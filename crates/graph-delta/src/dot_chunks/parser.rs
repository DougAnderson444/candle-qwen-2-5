//! This module provides functionality to parse DOT files into structured chunks
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Other error: {0}")]
    Other(String),
    /// From pest::error::Error<Rule>>
    #[error(transparent)]
    PestError(#[from] pest::error::Error<Rule>),
}

#[derive(Parser)]
#[grammar = "dot_chunks/dot.pest"]
pub struct DotParser;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Chunk {
    /// Node, edge, subgraph, attr_stmt, id_eq, rank
    pub kind: String,
    /// Identifier (for nodes, subgraphs, attr_stmt)
    pub id: Option<String>,
    /// Attributes map
    #[serde(default)]
    pub attrs: HashMap<String, String>,
    /// Line number range in the original DOT file
    pub range: (usize, usize),
    /// Extra info, e.g., for edges, the target node. For id_eq, the value.
    pub extra: Option<String>,
}

/// Formats a HashMap of attributes into a DOT attribute string.
fn format_dot_attributes(attrs: &HashMap<String, String>) -> String {
    attrs
        .iter()
        .map(|(k, v)| {
            // Per DOT language spec, identifiers that are not simple alphanumeric
            // must be quoted. HTML-like labels start with '<' and must not be quoted.
            if v.starts_with('<') && v.ends_with('>') {
                format!("{}={}", k, v)
            } else if v.chars().any(|c| !c.is_alphanumeric()) || v.is_empty() {
                format!(r#"{}="{}""#, k, v.replace('"', r#"\""#))
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// Parses a string of DOT attributes into a HashMap.
pub fn parse_attribute_string(s: &str) -> HashMap<String, String> {
    match DotParser::parse(Rule::a_list, s) {
        Ok(mut pairs) => parse_dot_attributes(pairs.next().unwrap()),
        Err(_) => HashMap::new(), // Return empty map on parsing error
    }
}

impl Chunk {
    /// Render this chunk back to DOT format
    pub fn to_dot(&self) -> String {
        let attrs_str = format_dot_attributes(&self.attrs);
        match self.kind.as_str() {
            "node" => {
                let id = self.id.as_deref().unwrap_or("unknown");
                if !self.attrs.is_empty() {
                    format!("    {} [{}];", id, attrs_str)
                } else {
                    format!("    {};", id)
                }
            }
            "bare_node" => {
                let id = self.id.as_deref().unwrap_or("unknown");
                format!("    {};", id)
            }
            "edge" => {
                let from = self.id.as_deref().unwrap_or("unknown");
                let to = self.extra.as_deref().unwrap_or("unknown");
                if !self.attrs.is_empty() {
                    format!("    {} -> {} [{}];", from, to, attrs_str)
                } else {
                    format!("    {} -> {};", from, to)
                }
            }
            "attr_stmt" => {
                let stmt_type = self.id.as_deref().unwrap_or("graph");
                if !self.attrs.is_empty() {
                    format!("    {} [{}];", stmt_type, attrs_str)
                } else {
                    format!("    {};", stmt_type)
                }
            }
            "id_eq" => {
                let key = self.id.as_deref().unwrap_or("unknown");
                let value = self.extra.as_deref().unwrap_or("\"\"");
                format!("    {} = {};", key, value)
            }
            "subgraph" => {
                if let Some(id) = &self.id {
                    format!("    subgraph {} {{", id)
                } else {
                    "    subgraph {".to_string()
                }
            }
            "rank" => {
                let rank_type = self.id.as_deref().unwrap_or("same");
                let nodes = self
                    .attrs
                    .get("nodes")
                    .cloned()
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("    {{ rank={}; {} }}", rank_type, nodes)
            }
            _ => format!("    // Unknown chunk type: {}", self.kind),
        }
    }
}

fn span_to_line_range(dot: &str, start: usize, end: usize) -> (usize, usize) {
    let start_line = dot[..start].matches('\n').count() + 1;
    let end_line = dot[..end].matches('\n').count() + 1;
    (start_line, end_line)
}

/// Recursively parses a pest `a_list` pair into a HashMap.
fn parse_dot_attributes(pair: Pair<Rule>) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    for item in pair.into_inner() {
        if let Rule::id_eq = item.as_rule() {
            let mut inner = item.into_inner();
            let key = inner.next().unwrap().as_str().to_string();
            let mut value = inner.next().unwrap().as_str().to_string();
            // Unquote the value if it's a quoted string
            if value.starts_with('"') && value.ends_with('"') {
                value = value[1..value.len() - 1].replace(r#"\""#, r#"""#);
            }
            attrs.insert(key, value);
        }
    }
    attrs
}

pub fn parse_dot_to_chunks(dot: &str) -> Result<Vec<Chunk>, Error> {
    let mut chunks = Vec::new();

    let file = DotParser::parse(Rule::dotfile, dot)?
        .next()
        .ok_or_else(|| {
            Error::ParseError("Failed to parse DOT file: no dotfile rule found".to_string())
        })?;

    fn walk(pair: Pair<Rule>, dot: &str, chunks: &mut Vec<Chunk>) {
        match pair.as_rule() {
            Rule::node_stmt => {
                let span = pair.as_span();
                let (start_line, end_line) = span_to_line_range(dot, span.start(), span.end());

                let mut inner = pair.into_inner();
                let node_id_pair = inner.next().unwrap();
                let id = node_id_pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .to_string();

                let attrs = inner
                    .next()
                    .and_then(|p| p.into_inner().next().map(parse_dot_attributes))
                    .unwrap_or_default();

                chunks.push(Chunk {
                    kind: "node".to_string(),
                    id: Some(id),
                    attrs,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            Rule::edge_stmt => {
                let span = pair.as_span();
                let (start_line, end_line) = span_to_line_range(dot, span.start(), span.end());

                let mut inner = pair.into_inner();
                let from_pair = inner.next().unwrap();
                let from = from_pair.as_str().trim().to_string();

                let mut targets = Vec::new();
                let mut attrs = HashMap::new();
                for p in inner {
                    match p.as_rule() {
                        Rule::edge_rhs => {
                            let target = p.into_inner().next().unwrap();
                            targets.push(target.as_str().trim().to_string());
                        }
                        Rule::attr_list => {
                            attrs = p
                                .into_inner()
                                .next()
                                .map(parse_dot_attributes)
                                .unwrap_or_default();
                        }
                        _ => {}
                    }
                }

                if let Some(to) = targets.first() {
                    chunks.push(Chunk {
                        kind: "edge".to_string(),
                        id: Some(from),
                        extra: Some(to.clone()),
                        attrs: attrs.clone(),
                        range: (start_line, end_line),
                    });
                }
                for i in 1..targets.len() {
                    chunks.push(Chunk {
                        kind: "edge".to_string(),
                        id: Some(targets[i - 1].clone()),
                        extra: Some(targets[i].clone()),
                        attrs: attrs.clone(),
                        range: (start_line, end_line),
                    });
                }
            }

            Rule::subgraph => {
                let span = pair.as_span();
                let (start_line, end_line) = span_to_line_range(dot, span.start(), span.end());

                let mut inner = pair.clone().into_inner();
                let id = inner
                    .find(|p| p.as_rule() == Rule::ident)
                    .map(|p| p.as_str().to_string());

                // Subgraphs can have attributes applied via an `attr_stmt` inside them,
                // but we will handle this via the interpreter applying updates.
                // Here we just create the subgraph chunk.

                chunks.push(Chunk {
                    kind: "subgraph".to_string(),
                    id,
                    attrs: HashMap::new(), // Placeholder, to be populated by interpreter if needed
                    range: (start_line, end_line),
                    extra: None,
                });

                for inner_pair in pair.into_inner() {
                    if inner_pair.as_rule() == Rule::stmt_list {
                        for stmt in inner_pair.into_inner() {
                            walk(stmt, dot, chunks);
                        }
                    }
                }
            }

            Rule::id_eq => {
                let span = pair.as_span();
                let (start_line, end_line) = span_to_line_range(dot, span.start(), span.end());

                let mut inner = pair.into_inner();
                let key = inner.next().map(|p| p.as_str().trim().to_string());
                let value = inner.next().map(|p| p.as_str().trim().to_string());

                chunks.push(Chunk {
                    kind: "id_eq".to_string(),
                    id: key,
                    attrs: HashMap::new(),
                    range: (start_line, end_line),
                    extra: value,
                });
            }

            Rule::attr_stmt => {
                let span = pair.as_span();
                let (start_line, end_line) = span_to_line_range(dot, span.start(), span.end());

                let mut inner = pair.into_inner();
                let stmt_type = inner.next().map(|p| p.as_str().trim().to_string());
                let attrs = inner
                    .next()
                    .and_then(|p| p.into_inner().next().map(parse_dot_attributes))
                    .unwrap_or_default();

                chunks.push(Chunk {
                    kind: "attr_stmt".to_string(),
                    id: stmt_type,
                    attrs,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            _ => {
                for inner in pair.into_inner() {
                    walk(inner, dot, chunks);
                }
            }
        }
    }

    walk(file, dot, &mut chunks);
    Ok(chunks)
}

pub fn chunks_to_dot(chunks: &[Chunk]) -> String {
    chunks_to_dot_with_indent(chunks, 0)
}

fn chunks_to_dot_with_indent(chunks: &[Chunk], indent_level: usize) -> String {
    let mut output = String::new();
    let indent = "    ";
    let indent_str = indent.repeat(indent_level);

    for chunk in chunks {
        match chunk.kind.as_str() {
            "subgraph" => {
                // Subgraph rendering is handled by the parent wrappers
                // to correctly handle nesting. Here we just add its attributes.
            }
            "rank" => {
                output.push_str(&indent_str);
                output.push_str(&chunk.to_dot());
                output.push('\n');
            }
            _ => {
                output.push_str(&indent_str);
                output.push_str(&chunk.to_dot());
                output.push('\n');
            }
        }
    }
    output
}

pub fn chunks_to_complete_dot(chunks: &[Chunk], graph_name: Option<&str>) -> String {
    // This function is a wrapper around chunks_to_dot_nested, which handles the full logic.

    chunks_to_dot_nested(chunks, graph_name)
}

pub fn chunks_to_dot_nested(chunks: &[Chunk], graph_name: Option<&str>) -> String {
    let mut output = String::new();
    let name = graph_name.unwrap_or("G");
    output.push_str(&format!("digraph {} {{\n", name));

    let mut sorted_chunks = chunks.to_vec();
    sorted_chunks.sort_by_key(|c| c.range.0);

    let mut subgraph_stack: Vec<(String, usize, usize)> = Vec::new();

    for chunk in &sorted_chunks {
        while let Some((_, _, end)) = subgraph_stack.last() {
            if chunk.range.0 > *end && *end != 0 {
                subgraph_stack.pop();
                let indent = "    ".repeat(subgraph_stack.len());
                output.push_str(&format!("{}}}}}\n", indent));
            } else {
                break;
            }
        }

        let indent = "    ".repeat(subgraph_stack.len() + 1);

        match chunk.kind.as_str() {
            "subgraph" => {
                let id_str = chunk.id.as_deref().unwrap_or("");
                let attrs_str = format_dot_attributes(&chunk.attrs);
                output.push_str(&format!("{}subgraph {} {{\n", indent, id_str));
                if !attrs_str.is_empty() {
                    output.push_str(&format!("{}    graph [{}];\n", indent, attrs_str));
                }
                subgraph_stack.push((id_str.to_string(), chunk.range.0, chunk.range.1));
            }
            "rank" => {
                output.push_str(&format!("{}{}\n", indent, chunk.to_dot()));
            }
            _ => {
                let chunk_str = chunk.to_dot().trim_start().to_string();
                output.push_str(&format!("{}{}\n", indent, chunk_str));
            }
        }
    }

    while !subgraph_stack.is_empty() {
        subgraph_stack.pop();
        let indent = "    ".repeat(subgraph_stack.len());
        output.push_str(&format!("{}}}}}\n", indent));
    }

    output.push_str("}\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_roundtrip() {
        let dot = r#"digraph G {
    A [label="Node A"];
    B [label="Node B"];
    A -> B [color="red"];
}"#;

        let chunks = parse_dot_to_chunks(dot).expect("Parse failed");
        let reconstructed = chunks_to_complete_dot(&chunks, Some("G"));

        println!("Original:\n{}", dot);
        println!("\nReconstructed:\n{}", reconstructed);

        let chunks2 = parse_dot_to_chunks(&reconstructed).expect("Reconstructed parse failed");
        assert_eq!(chunks.len(), chunks2.len(), "Chunk count should match");

        let node_a = chunks
            .iter()
            .find(|c| c.id.as_deref() == Some("A"))
            .unwrap();
        assert_eq!(node_a.attrs.get("label"), Some(&"Node A".to_string()));
    }

    #[test]
    fn test_roundtrip_preserves_structure() {
        let dot = r#"digraph Test {
    node1 [color="blue"];
    node2 [shape="box"];
    node1 -> node2 [label="edge1"];
}"#;

        let chunks = parse_dot_to_chunks(dot).expect("Parse failed");
        let reconstructed = chunks_to_complete_dot(&chunks, Some("Test"));

        assert!(reconstructed.contains("node1"));
        assert!(reconstructed.contains("node2"));
        assert!(reconstructed.contains("node1 -> node2"));
        assert!(reconstructed.contains("color=blue"));
        assert!(reconstructed.contains("shape=box"));
    }

    #[test]
    fn test_attribute_parsing() {
        let attrs_str = r#"label="Node \"A\"", color=red, style=dashed"#;
        let attrs = parse_attribute_string(attrs_str);
        assert_eq!(attrs.get("label"), Some(&"Node \"A\"".to_string()));
        assert_eq!(attrs.get("color"), Some(&"red".to_string()));
        assert_eq!(attrs.get("style"), Some(&"dashed".to_string()));
    }

    #[test]
    fn test_parse_dot_to_chunks_kitchen_sink() {
        let dot = std::fs::read_to_string("./tests/fixtures/kitchen_sink.dot")
            .expect("Failed to read kitchen_sink.dot");

        let chunks = parse_dot_to_chunks(&dot).expect("Parse failed");

        println!("\n=== Found {} chunks ===", chunks.len());
        chunks.iter().for_each(|c| println!("{:?}", c));

        assert!(
            chunks.len() >= 10,
            "Expected at least 10 chunks, got {}",
            chunks.len()
        );

        let a1_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("A1") && !c.attrs.is_empty())
            .unwrap();
        assert!(!a1_node.attrs.is_empty(), "A1 should have attributes");
        assert_eq!(
            a1_node.attrs.get("label"),
            Some(&"A1: internal link".to_string())
        );
        assert_eq!(a1_node.attrs.get("URL"), Some(&"/blog/69".to_string()));

        let a2_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("A2") && !c.attrs.is_empty())
            .unwrap();
        assert!(
            a2_node.attrs.get("tooltip").is_some(),
            "A2 should have tooltip attribute"
        );

        let rec_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("RecNode"))
            .unwrap();
        assert_eq!(rec_node.attrs.get("shape"), Some(&"record".to_string()));

        let a1_edges: Vec<_> = chunks
            .iter()
            .filter(|c| c.kind == "edge" && c.id.as_deref() == Some("A1"))
            .collect();
        assert!(!a1_edges.is_empty(), "Missing edges from A1");

        let subgraphs: Vec<_> = chunks.iter().filter(|c| c.kind == "subgraph").collect();
        assert!(subgraphs.len() >= 2, "Expected at least 2 subgraphs");

        let outer_cluster = subgraphs
            .iter()
            .find(|c| c.id.as_deref() == Some("cluster_Outer"))
            .unwrap();
        // In this model, subgraph attributes are graph attributes inside the subgraph scope.
        // So we look for a separate attr_stmt chunk.
        let outer_attrs = chunks
            .iter()
            .find(|c| {
                c.kind == "attr_stmt"
                    && c.id.as_deref() == Some("graph")
                    && c.range.0 > outer_cluster.range.0
                    && c.range.1 < outer_cluster.range.1
            })
            .unwrap();
        assert_eq!(
            outer_attrs.attrs.get("label"),
            Some(&"Outer Cluster".to_string())
        );
    }
}
