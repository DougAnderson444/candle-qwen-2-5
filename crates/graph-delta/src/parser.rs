//! This module provides functionality to parse DOT files into structured chunks
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[grammar = "dot.pest"]
pub struct DotParser;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chunk {
    pub kind: String, // "node", "edge", "subgraph", "attr_assign"
    pub id: Option<String>,
    pub attrs: Option<String>,
    pub range: (usize, usize), // line numbers
    pub extra: Option<String>, // for edge target, etc.
}

fn span_to_line_range(dot: &str, start: usize, end: usize) -> (usize, usize) {
    let start_line = dot[..start].matches('\n').count() + 1;
    let end_line = dot[..end].matches('\n').count() + 1;
    (start_line, end_line)
}

pub fn parse_dot_to_chunks(dot: &str) -> Result<Vec<Chunk>, String> {
    let mut chunks = Vec::new();

    let file = DotParser::parse(Rule::file, dot)
        .map_err(|e| format!("Parse error: {}", e))?
        .next()
        .ok_or("No parse tree")?;

    fn walk(pair: Pair<Rule>, dot: &str, chunks: &mut Vec<Chunk>) {
        match pair.as_rule() {
            Rule::node_stmt => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                let mut inner = pair.into_inner();
                let id = inner.next().map(|p| p.as_str().to_string());

                // Skip optional port
                let mut next = inner.next();
                if let Some(ref p) = next {
                    if p.as_rule() == Rule::port {
                        next = inner.next(); // Move to attr_list
                    }
                }

                let attrs = next.and_then(|p| {
                    if p.as_rule() == Rule::attr_list {
                        p.into_inner().next().map(|a| a.as_str().to_string())
                    } else {
                        None
                    }
                });

                chunks.push(Chunk {
                    kind: "node".to_string(),
                    id,
                    attrs,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            Rule::edge_stmt => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                let mut inner = pair.into_inner();

                // First node
                let from = inner.next().map(|p| p.as_str().to_string());

                // Collect all edge targets (could be chain: A -> B -> C)
                let mut targets = Vec::new();
                let mut attrs = None;

                for p in inner {
                    match p.as_rule() {
                        Rule::edge_rhs => {
                            // Extract target node from edge_rhs
                            for rhs_inner in p.into_inner() {
                                if rhs_inner.as_rule() == Rule::ident {
                                    targets.push(rhs_inner.as_str().to_string());
                                    break;
                                }
                            }
                        }
                        Rule::attr_list => {
                            attrs = p.into_inner().next().map(|a| a.as_str().to_string());
                        }
                        _ => {}
                    }
                }

                // Create edge for first target
                if let Some(to) = targets.first() {
                    chunks.push(Chunk {
                        kind: "edge".to_string(),
                        id: from.clone(),
                        attrs: attrs.clone(),
                        range: (start_line, end_line),
                        extra: Some(to.clone()),
                    });
                }

                // For edge chains (A -> B -> C), create separate edges
                for i in 1..targets.len() {
                    chunks.push(Chunk {
                        kind: "edge".to_string(),
                        id: Some(targets[i - 1].clone()),
                        attrs: attrs.clone(),
                        range: (start_line, end_line),
                        extra: Some(targets[i].clone()),
                    });
                }
            }

            Rule::subgraph => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                // Try to find subgraph name
                let mut id = None;
                for inner in pair.clone().into_inner() {
                    if inner.as_rule() == Rule::ident {
                        id = Some(inner.as_str().to_string());
                        break;
                    }
                }

                chunks.push(Chunk {
                    kind: "subgraph".to_string(),
                    id,
                    attrs: None,
                    range: (start_line, end_line),
                    extra: None,
                });

                // Recurse into subgraph body
                for inner in pair.into_inner() {
                    walk(inner, dot, chunks);
                }
            }

            Rule::attr_assign => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                let mut inner = pair.into_inner();
                let key = inner.next().map(|p| p.as_str().to_string());
                let value = inner.next().map(|p| p.as_str().to_string());

                chunks.push(Chunk {
                    kind: "attr_assign".to_string(),
                    id: key,
                    attrs: value,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            Rule::bare_node => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                let id = pair.into_inner().next().map(|p| p.as_str().to_string());

                chunks.push(Chunk {
                    kind: "bare_node".to_string(),
                    id,
                    attrs: None,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            _ => {
                // Recurse for other rules
                for inner in pair.into_inner() {
                    walk(inner, dot, chunks);
                }
            }
        }
    }

    walk(file, dot, &mut chunks);
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dot_to_chunks_kitchen_sink() {
        let dot = std::fs::read_to_string("./tests/fixtures/kitchen_sink.dot")
            .expect("Failed to read kitchen_sink.dot");

        let chunks = parse_dot_to_chunks(&dot).expect("Parse failed");

        println!("Found {} chunks", chunks.len());
        for chunk in &chunks {
            println!("{:?}", chunk);
        }

        // Expect at least 10 chunks (nodes, edges, subgraphs, etc.)
        assert!(
            chunks.len() >= 10,
            "Expected at least 10 chunks, got {}",
            chunks.len()
        );

        // Check for some known node IDs
        assert!(
            chunks.iter().any(|c| {
                (c.kind == "node" || c.kind == "bare_node") && c.id.as_deref() == Some("A1")
            }),
            "Missing node A1"
        );

        // Check for edges from A1
        assert!(
            chunks
                .iter()
                .any(|c| { c.kind == "edge" && c.id.as_deref() == Some("A1") }),
            "Missing edge from A1"
        );

        // Check for subgraph
        assert!(
            chunks.iter().any(|c| c.kind == "subgraph"),
            "Missing subgraph"
        );
    }
}
