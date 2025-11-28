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
    /// Node, edge, subgraph, attr_assign
    pub kind: String,
    /// Identifier (for nodes, subgraphs, attr_assign)
    pub id: Option<String>,
    /// Attributes as a raw string
    pub attrs: Option<String>,
    /// Line number range in the original DOT file
    pub range: (usize, usize),
    /// Extra info, e.g., for edges, the target node
    pub extra: Option<String>,
}

impl Chunk {
    /// Render this chunk back to DOT format
    pub fn to_dot(&self) -> String {
        match self.kind.as_str() {
            "node" => {
                let id = self.id.as_deref().unwrap_or("unknown");
                if let Some(attrs) = &self.attrs {
                    format!("    {} [{}];", id, attrs)
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
                if let Some(attrs) = &self.attrs {
                    format!("    {} -> {} [{}];", from, to, attrs)
                } else {
                    format!("    {} -> {};", from, to)
                }
            }
            "attr_assign" => {
                let key = self.id.as_deref().unwrap_or("unknown");
                let value = self.attrs.as_deref().unwrap_or("\"\"");
                format!("    {} = {};", key, value)
            }
            "subgraph" => {
                if let Some(id) = &self.id {
                    format!("    subgraph {} {{\n    }}", id)
                } else {
                    "    subgraph {\n    }".to_string()
                }
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

pub fn parse_dot_to_chunks(dot: &str) -> Result<Vec<Chunk>, String> {
    let mut chunks = Vec::new();

    let file = DotParser::parse(Rule::dotfile, dot)
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

                // First element is node_id
                let node_id = inner.next();
                let id = node_id.as_ref().and_then(|p| {
                    // node_id contains ident (and optional port)
                    p.clone()
                        .into_inner()
                        .next()
                        .map(|ident| ident.as_str().to_string())
                });

                // Next element (if present) is attr_list
                let attrs = inner.next().and_then(|p| {
                    if p.as_rule() == Rule::attr_list {
                        // Get the first a_list inside attr_list
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

                // First element is either subgraph or node_id
                let from_pair = inner.next();
                let from = from_pair.as_ref().map(|p| {
                    match p.as_rule() {
                        Rule::node_id => {
                            // For node_id, preserve the entire string including port
                            // node_id = ident ~ port?
                            p.as_str().trim().to_string()
                        }
                        Rule::subgraph => {
                            // For subgraph, try to get its identifier
                            p.clone()
                                .into_inner()
                                .find(|inner| inner.as_rule() == Rule::ident)
                                .map(|ident| ident.as_str().to_string())
                                .unwrap_or_else(|| "(anonymous)".to_string())
                        }
                        _ => p.as_str().trim().to_string(),
                    }
                });

                // Collect all edge targets and attrs
                let mut targets = Vec::new();
                let mut attrs = None;

                for p in inner {
                    match p.as_rule() {
                        Rule::edge_rhs => {
                            // edge_rhs contains (subgraph | node_id) followed by optional edge_rhs
                            for rhs_inner in p.into_inner() {
                                match rhs_inner.as_rule() {
                                    Rule::node_id => {
                                        // Preserve entire node_id string including port
                                        targets.push(rhs_inner.as_str().to_string());
                                    }
                                    Rule::subgraph => {
                                        // For subgraph, get identifier or use anonymous
                                        let name = rhs_inner
                                            .into_inner()
                                            .find(|inner| inner.as_rule() == Rule::ident)
                                            .map(|ident| ident.as_str().to_string())
                                            .unwrap_or_else(|| "(anonymous)".to_string());
                                        targets.push(name);
                                    }
                                    Rule::edge_rhs => {
                                        // Don't process nested edge_rhs here, it will be processed in outer loop
                                        break;
                                    }
                                    _ => {}
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
                        extra: Some(to.trim().to_string()),
                    });
                }

                // For edge chains (A -> B -> C), create separate edges
                for i in 1..targets.len() {
                    chunks.push(Chunk {
                        kind: "edge".to_string(),
                        id: Some(targets[i - 1].clone()),
                        attrs: attrs.clone(),
                        range: (start_line, end_line),
                        extra: Some(targets[i].trim().to_string()),
                    });
                }
            }

            Rule::subgraph => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                // Try to find subgraph name (ident after optional "subgraph" keyword)
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

            Rule::id_eq => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                let mut inner = pair.into_inner();
                let key = inner.next().map(|p| p.as_str().trim().to_string());
                let value = inner.next().map(|p| p.as_str().trim().to_string());

                chunks.push(Chunk {
                    kind: "id_eq".to_string(),
                    id: key,
                    attrs: value,
                    range: (start_line, end_line),
                    extra: None,
                });
            }

            Rule::attr_stmt => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);

                // attr_stmt is: (graph | node | edge) ~ attr_list
                let mut inner = pair.into_inner();
                let stmt_type = inner.next().map(|p| p.as_str().trim().to_string());

                // Collect all attributes from attr_list
                let attrs = inner.next().and_then(|p| {
                    if p.as_rule() == Rule::attr_list {
                        // Get first a_list
                        p.into_inner().next().map(|a| a.as_str().to_string())
                    } else {
                        None
                    }
                });

                chunks.push(Chunk {
                    kind: "attr_stmt".to_string(),
                    id: stmt_type,
                    attrs,
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

/// Reconstruct a DOT file from chunks
/// Note: This is a simplified reconstruction that won't perfectly preserve
/// the original structure (subgraph nesting, comments, formatting), but will
/// produce valid DOT that represents the same graph structure.
pub fn chunks_to_dot(chunks: &[Chunk]) -> String {
    chunks_to_dot_with_indent(chunks, 0)
}

fn chunks_to_dot_with_indent(chunks: &[Chunk], indent_level: usize) -> String {
    let mut output = String::new();
    let indent = "    ";

    let mut i = 0;
    while i < chunks.len() {
        let chunk = &chunks[i];
        let indent_str = indent.repeat(indent_level);

        match chunk.kind.as_str() {
            "node" => {
                if let Some(ref id) = chunk.id {
                    output.push_str(&indent_str);
                    output.push_str(id);

                    if let Some(ref attrs) = chunk.attrs {
                        output.push_str(" [");
                        output.push_str(attrs);
                        output.push(']');
                    }

                    output.push_str(";\n");
                }
                i += 1;
            }

            "edge" => {
                if let (Some(from), Some(to)) = (&chunk.id, &chunk.extra) {
                    output.push_str(&indent_str);
                    output.push_str(from);
                    output.push_str(" -> ");
                    output.push_str(to);

                    if let Some(ref attrs) = chunk.attrs {
                        output.push_str(" [");
                        output.push_str(attrs);
                        output.push(']');
                    }

                    output.push_str(";\n");
                }
                i += 1;
            }

            "subgraph" => {
                output.push_str(&indent_str);

                if let Some(ref id) = chunk.id {
                    output.push_str("subgraph ");
                    output.push_str(id);
                    output.push_str(" {\n");
                } else {
                    // Anonymous subgraph
                    output.push_str("{\n");
                }

                // Find all children of this subgraph (based on line ranges)
                let subgraph_start = chunk.range.0;
                let subgraph_end = chunk.range.1;
                let mut child_chunks = Vec::new();

                i += 1;
                while i < chunks.len() {
                    let potential_child = &chunks[i];
                    // Child chunks must be within the subgraph's line range
                    if potential_child.range.0 > subgraph_start
                        && potential_child.range.1 <= subgraph_end
                    {
                        child_chunks.push(potential_child.clone());
                        i += 1;
                    } else {
                        break;
                    }
                }

                // Recursively output children with increased indent
                if !child_chunks.is_empty() {
                    output.push_str(&chunks_to_dot_with_indent(&child_chunks, indent_level + 1));
                }

                output.push_str(&indent_str);
                output.push_str("}\n\n");
            }

            "id_eq" => {
                if let (Some(key), Some(value)) = (&chunk.id, &chunk.attrs) {
                    output.push_str(&indent_str);
                    output.push_str(key);
                    output.push_str(" = ");
                    output.push_str(value);
                    output.push_str(";\n");
                }
                i += 1;
            }

            "attr_stmt" => {
                if let Some(ref stmt_type) = chunk.id {
                    output.push_str(&indent_str);
                    output.push_str(stmt_type); // "graph", "node", or "edge"

                    if let Some(ref attrs) = chunk.attrs {
                        output.push_str(" [");
                        output.push_str(attrs);
                        output.push(']');
                    }

                    output.push_str(";\n");
                }
                i += 1;
            }

            _ => {
                // Unknown chunk type, skip
                i += 1;
            }
        }
    }

    output
}

// Helper function to wrap in digraph if needed
pub fn chunks_to_complete_dot(chunks: &[Chunk], graph_name: Option<&str>) -> String {
    let mut output = String::new();

    output.push_str("digraph ");
    if let Some(name) = graph_name {
        output.push('"');
        output.push_str(name);
        output.push('"');
    }
    output.push_str(" {\n");

    // Start with indent level 1 since we're inside the digraph
    output.push_str(&chunks_to_dot_with_indent(chunks, 1));

    output.push_str("}\n");
    output
}

/// A more sophisticated reconstruction that preserves subgraph structure
/// by using the range information to determine nesting
pub fn chunks_to_dot_nested(chunks: &[Chunk], graph_name: Option<&str>) -> String {
    let mut output = String::new();

    // Start graph
    let name = graph_name.unwrap_or("G");
    output.push_str(&format!("digraph {} {{\n", name));

    // Sort chunks by start line to process in order
    let mut sorted_chunks = chunks.to_vec();
    sorted_chunks.sort_by_key(|c| c.range.0);

    // Track subgraph stack with ranges
    let mut subgraph_stack: Vec<(String, usize, usize)> = Vec::new();

    for chunk in &sorted_chunks {
        // Close any subgraphs that have ended
        while let Some((_, _, end)) = subgraph_stack.last() {
            if chunk.range.0 > *end {
                subgraph_stack.pop();
                let indent = "    ".repeat(subgraph_stack.len() + 1);
                output.push_str(&format!("{}}}}}\n", indent));
            } else {
                break;
            }
        }

        let indent = "    ".repeat(subgraph_stack.len() + 1);

        match chunk.kind.as_str() {
            "subgraph" => {
                let id = chunk.id.as_deref().unwrap_or("");
                output.push_str(&format!("{}subgraph {} {{\n", indent, id));
                subgraph_stack.push((id.to_string(), chunk.range.0, chunk.range.1));
            }
            _ => {
                let chunk_str = chunk.to_dot().trim_start().to_string();
                output.push_str(&format!("{}{}\n", indent, chunk_str));
            }
        }
    }

    // Close remaining subgraphs
    while !subgraph_stack.is_empty() {
        subgraph_stack.pop();
        let indent = "    ".repeat(subgraph_stack.len() + 1);
        output.push_str(&format!("{}}}}}\n", indent));
    }

    // Close graph
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

        // Parse the reconstructed version to ensure it's valid
        let chunks2 = parse_dot_to_chunks(&reconstructed).expect("Reconstructed parse failed");

        assert_eq!(chunks.len(), chunks2.len(), "Chunk count should match");
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

        // Verify all nodes are present
        assert!(reconstructed.contains("node1"));
        assert!(reconstructed.contains("node2"));
        assert!(reconstructed.contains("node1 -> node2"));

        // Verify attributes are preserved
        assert!(reconstructed.contains("color=\"blue\"") || reconstructed.contains("color=blue"));
        assert!(reconstructed.contains("shape=\"box\"") || reconstructed.contains("shape=box"));
    }

    #[test]
    fn test_parse_dot_to_chunks_kitchen_sink() {
        let dot = std::fs::read_to_string("./tests/fixtures/kitchen_sink.dot")
            .expect("Failed to read kitchen_sink.dot");

        let chunks = parse_dot_to_chunks(&dot).expect("Parse failed");

        println!("\n=== Found {} chunks ===", chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            println!("{}: {:?}", i, chunk);
        }

        // Count chunk types
        let node_count = chunks.iter().filter(|c| c.kind == "node").count();
        let bare_node_count = chunks.iter().filter(|c| c.kind == "bare_node").count();
        let edge_count = chunks.iter().filter(|c| c.kind == "edge").count();
        let subgraph_count = chunks.iter().filter(|c| c.kind == "subgraph").count();
        let attr_assign_count = chunks.iter().filter(|c| c.kind == "attr_assign").count();

        println!("\n=== Chunk Statistics ===");
        println!("Nodes: {}", node_count);
        println!("Bare nodes: {}", bare_node_count);
        println!("Edges: {}", edge_count);
        println!("Subgraphs: {}", subgraph_count);
        println!("Attr assigns: {}", attr_assign_count);

        // Expect at least 10 chunks (nodes, edges, subgraphs, etc.)
        assert!(
            chunks.len() >= 10,
            "Expected at least 10 chunks, got {}",
            chunks.len()
        );

        // Check for known node IDs with attributes
        let a1_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("A1"));
        assert!(a1_node.is_some(), "Missing node A1");
        if let Some(node) = a1_node {
            println!("\nA1 node attributes: {:?}", node.attrs);
            assert!(node.attrs.is_some(), "A1 should have attributes");
        }

        // Check for A2 with tooltip attribute
        let a2_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("A2"));
        assert!(a2_node.is_some(), "Missing node A2");
        if let Some(node) = a2_node {
            let attrs = node.attrs.as_ref().unwrap();
            assert!(
                attrs.contains("tooltip"),
                "A2 should have tooltip attribute"
            );
        }

        // Check for RecNode with record shape
        let rec_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("RecNode"));
        assert!(rec_node.is_some(), "Missing RecNode");
        if let Some(node) = rec_node {
            let attrs = node.attrs.as_ref().unwrap();
            assert!(
                attrs.contains("shape=record"),
                "RecNode should have shape=record"
            );
        }

        // Check for edges from A1
        let a1_edges: Vec<_> = chunks
            .iter()
            .filter(|c| c.kind == "edge" && c.id.as_deref() == Some("A1"))
            .collect();
        assert!(!a1_edges.is_empty(), "Missing edges from A1");
        println!("\nEdges from A1: {}", a1_edges.len());
        for edge in &a1_edges {
            println!("  A1 -> {:?}", edge.extra);
        }

        // Check for edge with port (A1 -> RecNode:p0)
        let port_edge = chunks.iter().find(|c| {
            c.kind == "edge"
                && c.id.as_deref() == Some("A1")
                && c.extra
                    .as_ref()
                    .map(|e| e.contains("RecNode"))
                    .unwrap_or(false)
        });
        assert!(port_edge.is_some(), "Missing edge from A1 to RecNode");

        // Check for subgraphs
        let subgraphs: Vec<_> = chunks.iter().filter(|c| c.kind == "subgraph").collect();
        assert!(
            subgraphs.len() >= 2,
            "Expected at least 2 subgraphs (outer + inner)"
        );
        println!("\nSubgraphs found:");
        for sg in &subgraphs {
            println!("  {:?} at lines {:?}", sg.id, sg.range);
        }

        // Check for cluster_Outer
        let outer_cluster = chunks.iter().find(|c| {
            c.kind == "subgraph"
                && c.id
                    .as_ref()
                    .map(|id| id.contains("Outer"))
                    .unwrap_or(false)
        });
        assert!(outer_cluster.is_some(), "Missing cluster_Outer subgraph");

        // Check for cluster_Inner
        let inner_cluster = chunks.iter().find(|c| {
            c.kind == "subgraph"
                && c.id
                    .as_ref()
                    .map(|id| id.contains("Inner"))
                    .unwrap_or(false)
        });
        assert!(inner_cluster.is_some(), "Missing cluster_Inner subgraph");

        // Check for graph-level attribute assignments
        let graph_attrs: Vec<_> = chunks.iter().filter(|c| c.kind == "attr_assign").collect();
        println!("\nGraph attributes: {}", graph_attrs.len());
        for attr in &graph_attrs {
            println!("  {} = {:?}", attr.id.as_ref().unwrap(), attr.attrs);
        }

        // Verify HTML-like label node
        let html_node = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("HTMLNode"));
        assert!(html_node.is_some(), "Missing HTMLNode");
        if let Some(node) = html_node {
            let attrs = node.attrs.as_ref().unwrap();
            assert!(
                attrs.contains("plaintext") || attrs.contains("table"),
                "HTMLNode should have HTML label"
            );
        }

        // Verify nodes with special styling
        let style_node1 = chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some("StyleNode1"));
        assert!(style_node1.is_some(), "Missing StyleNode1");
        if let Some(node) = style_node1 {
            let attrs = node.attrs.as_ref().unwrap();
            assert!(
                attrs.contains("penwidth"),
                "StyleNode1 should have penwidth"
            );
        }

        println!("\n=== All assertions passed! ===\n");
    }
}
