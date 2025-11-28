//! Optimized for small models (0.5B/1.5B), now with intelligent,
//! attribute-preserving updates for both nodes and edges.
//!
//! Run:
//! ```sh
//! cargo run --release --example simple_llm_editor --features graph-delta/llm
//! ```
use anyhow::{Result, anyhow};
use regex::Regex;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

use graph_delta::{
    commands::{DotCommand, apply_command},
    parser::{Chunk, chunks_to_complete_dot, parse_dot_to_chunks},
};

use candle_qwen2_5_core::{ModelArgs, Qwen2Model, Which};

// --- Main Application ---

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Simple LLM Graph Editor (w/ Full Update Logic) ===\n");

    // 1. Load graph with attributes to test preservation
    let initial_dot = r#"
    digraph G {
        A [label="Node A" color=blue];
        B [label="Node B" shape=box];
        A -> B [label="Original Edge"];
}
"#;

    let mut chunks = parse_dot_to_chunks(initial_dot)
        .map_err(|e| anyhow::anyhow!("Failed to parse DOT: {}", e))?;
    println!("Initial Graph:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("G")));

    // 2. User request to test edge update and creation with attributes
    let user_request =
        "make the edge from A to B red and add a new edge from B back to A, labelled 'reverse'";
    println!("User: \"{}\"\n", user_request);

    // 3. Initialize model
    let model_args = ModelArgs {
        cpu: true,
        which: Which::W25_0_5b,
        ..Default::default()
    };
    let mut model = Qwen2Model::new(&model_args).await?;
    let start_time = Instant::now();

    // 4. Use prompt that now includes edge updates and attributes
    let prompt = format!(
        r#"You are a text analysis bot. Your ONLY job is to convert a user request into simple action lines.
Respond with ONLY the precise action lines. No explanations, no markdown, no quotes, no dashes.

Q: add node X with label \"My Node\"
A: node: X, \"My Node\"

Q: connect A to B with label \"link\"
A: edge: A, B, label=\"link\"

Q: Change the label of node B to \"New B\"
A: update_node: B, label=\"New B\"

Q: make the edge from A to B red
A: update_edge: A, B, color=red

Q: {}\nA:"#,
        user_request
    );

    println!("--- LLM Response ---");
    let mut llm_response = String::new();
    model.generate(&prompt, 128, |s| {
        print!("{}", s);
        std::io::stdout().flush()?;
        llm_response.push_str(&s);
        Ok(())
    })?;
    println!("\n");

    // 5. Parse LLM response and apply actions directly
    println!("--- Applying Actions ---");
    parse_and_apply_actions(&llm_response, &mut chunks)?;

    // 6. Show result
    let modified_dot = chunks_to_complete_dot(&chunks, Some("G"));
    println!("\n--- Modified Graph ---");
    println!("{}", modified_dot);
    println!("\nExecution time: {:?}", start_time.elapsed());

    Ok(())
}

// --- "Brains in Rust" Functions ---

/// A simple parser for Graphviz-style attribute strings like `key="value" key2=value2`.
fn parse_attrs(attrs_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let re = Regex::new(r#"(?P<key>\w+)\s*=\s*(?:"(?P<qval>[^"]*)"|(?P<val>[^\s,]+))"#).unwrap();
    for caps in re.captures_iter(attrs_str) {
        let key = caps.name("key").unwrap().as_str().to_string();
        let value = caps
            .name("qval")
            .or_else(|| caps.name("val"))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        map.insert(key, value);
    }
    map
}

/// Rebuilds an attribute string from a map, ensuring values are quoted.
fn build_attrs_string(attrs_map: &HashMap<String, String>) -> String {
    attrs_map
        .iter()
        .map(|(k, v)| format!(r#"{}=\"{}\""#, k, v))
        .collect::<Vec<_>>()
        .join(" ")
}

/// New parser that also contains the "brain" logic to apply commands for nodes and edges.
fn parse_and_apply_actions(response: &str, chunks: &mut Vec<Chunk>) -> Result<()> {
    let node_re = Regex::new(r"node:\s*([^,]+)(?:,\s*(.*))?")?;
    let edge_re = Regex::new(r"edge:\s*([^,]+),\s*([^,]+)(?:,\s*(.*))?")?;
    let update_node_re = Regex::new(r"update_node:\s*([^,]+),\s*(.+)")?;
    let update_edge_re = Regex::new(r"update_edge:\s*([^,]+),\s*([^,]+),\s*(.+)")?;

    for line in response.lines() {
        let clean_line = line.trim();

        if let Some(caps) = update_node_re.captures(clean_line) {
            let id = caps.get(1).unwrap().as_str().trim().to_string();
            let new_attrs_str = caps.get(2).unwrap().as_str().trim();

            let existing_chunk = chunks
                .iter_mut()
                .find(|c| c.kind == "node" && c.id.as_deref() == Some(&id))
                .ok_or_else(|| anyhow!("Node '{}' not found to update.", id))?;

            let mut attrs_map = parse_attrs(existing_chunk.attrs.as_deref().unwrap_or(""));
            attrs_map.extend(parse_attrs(new_attrs_str));
            let final_attrs = build_attrs_string(&attrs_map);

            let cmd = DotCommand::UpdateNode {
                id,
                attrs: Some(final_attrs),
            };
            println!("  Applying Intelligent Update: {:?}", cmd);
            apply_command(chunks, &cmd).map_err(|e| anyhow!(e))?;
        } else if let Some(caps) = update_edge_re.captures(clean_line) {
            let from = caps.get(1).unwrap().as_str().trim().to_string();
            let to = caps.get(2).unwrap().as_str().trim().to_string();
            let new_attrs_str = caps.get(3).unwrap().as_str().trim();

            let existing_chunk = chunks
                .iter_mut()
                .find(|c| {
                    c.kind == "edge"
                        && c.id.as_deref() == Some(&from)
                        && c.extra.as_deref() == Some(&to)
                })
                .ok_or_else(|| anyhow!("Edge from '{}' to '{}' not found to update.", from, to))?;

            let mut attrs_map = parse_attrs(existing_chunk.attrs.as_deref().unwrap_or(""));
            attrs_map.extend(parse_attrs(new_attrs_str));
            let final_attrs = build_attrs_string(&attrs_map);

            let cmd = DotCommand::UpdateEdge {
                from,
                to,
                attrs: Some(final_attrs),
            };
            println!("  Applying Intelligent Edge Update: {:?}", cmd);
            apply_command(chunks, &cmd).map_err(|e| anyhow!(e))?;
        } else if let Some(caps) = node_re.captures(clean_line) {
            let id = caps.get(1).unwrap().as_str().trim().to_string();
            let label = caps.get(2).map_or(id.clone(), |m| {
                m.as_str().trim().trim_matches('"').to_string()
            });
            let cmd = DotCommand::CreateNode {
                id,
                attrs: Some(format!("label=\"{}\"", label)),
                parent: None,
            };
            println!("  Applying CreateNode: {:?}", cmd);
            apply_command(chunks, &cmd).map_err(|e| anyhow!(e))?;
        } else if let Some(caps) = edge_re.captures(clean_line) {
            let from = caps.get(1).unwrap().as_str().trim().to_string();
            let to = caps.get(2).unwrap().as_str().trim().to_string();
            let attrs = caps.get(3).map(|m| m.as_str().trim().to_string());
            let cmd = DotCommand::CreateEdge {
                from,
                to,
                attrs,
                parent: None,
            };
            println!("  Applying CreateEdge: {:?}", cmd);
            apply_command(chunks, &cmd).map_err(|e| anyhow!(e))?;
        }
    }
    Ok(())
}

