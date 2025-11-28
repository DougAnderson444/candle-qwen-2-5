//! Optimized for small models (0.5B/1.5B) with minimal output requirements
//!
//! Run:
//! ```sh
//! cargo run --release --example simple_llm_editor --features graph-delta/llm
//! ```
use regex::Regex;
use anyhow::{anyhow, Result};
use std::io::Write;

use graph_delta::{
    commands::{apply_command, DotCommand},
    parser::{chunks_to_complete_dot, parse_dot_to_chunks},
};

use candle_qwen2_5_core::{ModelArgs, Qwen2Model, Which};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Simple LLM Graph Editor (Qwen 0.5B/1.5B) ===\n");

    // 1. Load graph
    let initial_dot = r#"
digraph G {
    A [label="Node A"];
    B [label="Node B"];
    A -> B;
}
"#;

    let mut chunks = parse_dot_to_chunks(initial_dot)
        .map_err(|e| anyhow::anyhow!("Failed to parse DOT: {}", e))?;
    println!("Initial Graph:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("G")));

    // 2. User request
    let user_request = "add node C and connect A to C";
    println!("User: \"{}\"\n", user_request);

    // 3. Initialize model
    let model_args = ModelArgs {
        cpu: true,
        which: Which::W25_0_5b,
        ..Default::default()
    };
    let mut model = Qwen2Model::new(&model_args).await?;

    // 4. Use VERY simple prompt optimized for small models
    let prompt = format!(
        r#"You are a text analysis bot. Your ONLY job is to convert a user request into simple action lines.
Respond with ONLY the precise action lines. No explanations, no markdown, no quotes, no dashes, no conversational text.

Q: add node X with label "My Node"
A: node: X, "My Node"

Q: connect A to B
A: edge: A, B

Q: add a new node Z and then connect it to Y
A: node: Z, "Node Z"
edge: Y, Z

Q: {}
A:"#,
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

    // 5. Parse simple action format
    println!("--- Parsing Actions ---");
    let commands = parse_simple_actions(&llm_response)?;

    // 6. Apply commands
    println!("--- Applying Commands ---");
    for cmd in &commands {
        println!("  {:?}", cmd);
        apply_command(&mut chunks, cmd)
            .map_err(|e| anyhow::anyhow!("Failed to apply command {:?}: {}", cmd, e))?;
    }

    // 7. Show result
    let modified_dot = chunks_to_complete_dot(&chunks, Some("G"));
    println!("\n--- Modified Graph ---");
    println!("{}", modified_dot);

    Ok(())
}

/// A robust, regex-based parser for the simple action format.
fn parse_simple_actions(response: &str) -> Result<Vec<DotCommand>> {
    let mut commands = Vec::new();
    // Regex for `node: ID, Label` (label is optional)
    let node_re = Regex::new(r#"node:\s*([^,]+)(?:,\s*(.*))?"#)?;
    // Regex for `edge: FROM, TO`
    let edge_re = Regex::new(r"edge:\s*([^,]+),\s*(.+)")?;

    for line in response.lines() {
        let mut clean_line = line.trim().trim_start_matches('-').trim().trim_matches('"');

        // Heuristic: The model often adds " to create a...". Find this and slice it off.
        if let Some(pos) = clean_line.find(" to ") {
            clean_line = &clean_line[..pos];
        }
        // Also trim any trailing quotes that might be left
        clean_line = clean_line.trim().trim_matches('"');

        if let Some(caps) = node_re.captures(clean_line) {
            let id = caps.get(1).unwrap().as_str().trim().to_string();
            // Use provided label, or create a default one. Trim quotes from label.
            let label = caps.get(2).and_then(|m| {
                let s = m.as_str().trim();
                if s.is_empty() || s == "label" { None } else { Some(s.trim_matches('"').to_string()) }
            }).unwrap_or_else(|| id.clone());

            commands.push(DotCommand::CreateNode {
                id,
                attrs: Some(format!("label=\"{}\"", label)),
                parent: None,
            });
        } else if let Some(caps) = edge_re.captures(clean_line) {
            let from = caps.get(1).unwrap().as_str().trim().to_string();
            let to = caps.get(2).unwrap().as_str().trim().to_string();
            commands.push(DotCommand::CreateEdge {
                from,
                to,
                attrs: None,
                parent: None,
            });
        }
    }

    if commands.is_empty() {
        return Err(anyhow!(
            "LLM response could not be parsed into any known action. Response: '{}'",
            response
        ));
    }

    Ok(commands)
}
