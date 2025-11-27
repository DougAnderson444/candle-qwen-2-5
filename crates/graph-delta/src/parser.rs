//! This module provides functionality to parse DOT files into structured chunks
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[grammar = "dot.pest"]
pub struct DotParser;

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub kind: String, // "node", "edge", "subgraph"
    pub id: Option<String>,
    pub attrs: Option<String>,
    pub range: (usize, usize), // line numbers
}

fn span_to_line_range(dot: &str, start: usize, end: usize) -> (usize, usize) {
    let start_line = dot[..start].lines().count();
    let end_line = dot[..end].lines().count();
    (start_line + 1, end_line + 1)
}

pub fn parse_dot_to_chunks(dot: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let file = DotParser::parse(Rule::file, dot)
        .expect("Failed to parse DOT")
        .next()
        .unwrap();

    fn walk(pair: Pair<Rule>, dot: &str, chunks: &mut Vec<Chunk>) {
        match pair.as_rule() {
            Rule::node_stmt => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);
                let mut inner = pair.into_inner();
                let id = inner.next().map(|p| p.as_str().to_string());
                let attrs = inner.next().map(|p| p.as_str().to_string());
                chunks.push(Chunk {
                    kind: "node".to_string(),
                    id,
                    attrs,
                    range: (start_line, end_line),
                });
            }
            Rule::edge_stmt => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);
                let mut inner = pair.into_inner();
                let from = inner.next().map(|p| p.as_str().to_string());
                let to = inner.next().map(|p| p.as_str().to_string());
                let attrs = inner.next().map(|p| p.as_str().to_string());
                chunks.push(Chunk {
                    kind: "edge".to_string(),
                    id: from, // Optionally store both from/to in attrs
                    attrs: Some(format!("to: {:?}, attrs: {:?}", to, attrs)),
                    range: (start_line, end_line),
                });
            }
            Rule::subgraph => {
                let span = pair.as_span();
                let (start, end) = (span.start(), span.end());
                let (start_line, end_line) = span_to_line_range(dot, start, end);
                let id = pair
                    .clone()
                    .into_inner()
                    .next()
                    .map(|p| p.as_str().to_string());
                chunks.push(Chunk {
                    kind: "subgraph".to_string(),
                    id,
                    attrs: None,
                    range: (start_line, end_line),
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
    chunks
}
