//! An example of using `graph-delta` with a simple, two-step LLM agent.
//!
//! This version is radically simple to work more reliably with smaller,
//! less powerful local models (e.g., Qwen2 0.5B).
//!
//! ## Example Workflow
//!
//! 1. Parse Graph: The application parses the DOT graph into memory first.
//! 2. LLM Call 1 (Simple Intent-to-Query): The LLM is given a very simple
//!    prompt and a single example. Its only job is to extract node IDs from the
//!    user's request into a simple JSON format: `{"tool":"find...","ids":[...]}`.
//! 3. Rust Executes Search: The application parses this simple JSON and
//!    searches the in-memory graph for the requested nodes.
//! 4. LLM Call 2 (Context-to-Command): The application sends a new, simple
//!    prompt to the LLM containing the user's request and the search results.
//!    The LLM's job is to generate the final `DotCommand` JSON.
//! 5. The final command is parsed and applied.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --release --example llm_editor --features graph-delta/llm
//! ```
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Instant;

use graph_delta::{
    commands::{DotCommand, apply_command},
    parser::{Chunk, chunks_to_complete_dot, parse_dot_to_chunks},
    tool::{execute_query_tool, get_system_prompt, get_tool_definitions, tool_call_to_command},
};

use candle_qwen2_5_core::{ModelArgs, Qwen2Model, Which};

#[derive(Debug, Serialize, Deserialize)]
struct ToolCall {
    name: String,
    parameters: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== LLM DOT Graph Editor (Tool Calling) ===\n");

    // 1. Load and parse the graph
    let initial_dot = include_str!("../tests/fixtures/simple_example.dot");
    let mut chunks = parse_dot_to_chunks(initial_dot)
        .map_err(|e| anyhow::anyhow!("Failed to parse DOT: {}", e))?;

    println!("Initial Graph:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("G")));

    // 2. User instruction
    let user_instruction = "Add a new node C with label 'Node C' and connect it to node A";
    println!("User: \"{}\"\n", user_instruction);

    // 3. Initialize model
    let model_args = ModelArgs {
        cpu: true,
        which: Which::W25_0_5b,
        ..Default::default()
    };
    let mut model = Qwen2Model::new(&model_args).await?;

    let start_time = Instant::now();

    // 4. Build prompt with tool definitions
    let tools = get_tool_definitions();
    let system_prompt = get_system_prompt();

    let prompt = format!(
        r#"{}

Available tools:
{}

User request: {}

Think step by step:
1. What nodes/edges need to be queried?
2. What modifications are needed?
3. Call the appropriate tools.

Respond with JSON tool calls in this format:
{{"name": "tool_name", "parameters": {{"param": "value"}}}}

Tool calls:"#,
        system_prompt,
        serde_json::to_string_pretty(&tools)?,
        user_instruction
    );

    println!("--- Querying LLM ---");
    let mut llm_response = String::new();
    model.generate(&prompt, 512, |s| {
        print!("{}", s);
        std::io::stdout().flush()?;
        llm_response.push_str(&s);
        Ok(())
    })?;
    println!("\n");

    // 5. Parse tool calls from response
    println!("--- Processing Tool Calls ---");
    let tool_calls = extract_tool_calls(&llm_response)?;

    let mut commands = Vec::new();

    for call in tool_calls {
        println!("Tool: {} with params: {}", call.name, call.parameters);

        // Check if it's a query tool or modification tool
        match call.name.as_str() {
            "get_node" | "list_nodes" | "get_edges" => {
                // Execute query and show results
                match execute_query_tool(&call.name, call.parameters, &chunks) {
                    Ok(result) => {
                        println!("  Result: {}", result);
                    }
                    Err(e) => {
                        println!("  Error: {}", e);
                    }
                }
            }
            _ => {
                // Convert to DotCommand
                match tool_call_to_command(&call.name, call.parameters) {
                    Ok(cmd) => {
                        println!("  -> Command: {:?}", cmd);
                        commands.push(cmd);
                    }
                    Err(e) => {
                        println!("  Error: {}", e);
                    }
                }
            }
        }
    }

    // 6. Apply commands
    println!("\n--- Applying Commands ---");
    for cmd in &commands {
        println!("Applying: {:?}", cmd);
        apply_command(&mut chunks, cmd).map_err(|e| anyhow::anyhow!("Failed to apply: {}", e))?;
    }

    // 7. Show final result
    let modified_dot = chunks_to_complete_dot(&chunks, Some("G"));
    println!("\n--- Modified Graph ---");
    println!("{}", modified_dot);
    println!("\nExecution time: {:?}", start_time.elapsed());

    Ok(())
}

/// Extract tool calls from LLM response
fn extract_tool_calls(response: &str) -> Result<Vec<ToolCall>> {
    let mut calls = Vec::new();

    // Try to extract JSON objects from response
    let cleaned = extract_json_from_markdown(response);

    // Try parsing as array first
    if let Ok(array) = serde_json::from_str::<Vec<ToolCall>>(cleaned) {
        return Ok(array);
    }

    // Try parsing as single object
    if let Ok(call) = serde_json::from_str::<ToolCall>(cleaned) {
        return Ok(vec![call]);
    }

    // Fallback: try to find JSON objects line by line
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(call) = serde_json::from_str::<ToolCall>(trimmed) {
                calls.push(call);
            }
        }
    }

    if calls.is_empty() {
        // If no valid JSON found, create a simplified parser
        calls = parse_simple_format(response)?;
    }

    Ok(calls)
}

/// Parse simple format like: "create_node A" or "connect A to B"
fn parse_simple_format(response: &str) -> Result<Vec<ToolCall>> {
    let mut calls = Vec::new();

    for line in response.lines() {
        let line = line.trim().to_lowercase();

        // Pattern: "create node X" or "add node X"
        if (line.contains("create") || line.contains("add")) && line.contains("node") {
            let words: Vec<&str> = line.split_whitespace().collect();
            if let Some(id) = words.last() {
                calls.push(ToolCall {
                    name: "create_node".to_string(),
                    parameters: serde_json::json!({
                        "id": id.to_uppercase(),
                        "label": format!("Node {}", id.to_uppercase())
                    }),
                });
            }
        }

        // Pattern: "connect A to B" or "edge from A to B"
        if line.contains("connect") || line.contains("edge") {
            let words: Vec<&str> = line.split_whitespace().collect();

            // Find "to" keyword
            if let Some(to_idx) = words.iter().position(|&w| w == "to") {
                if to_idx > 0 && to_idx < words.len() - 1 {
                    let from = words[to_idx - 1].to_uppercase();
                    let to = words[to_idx + 1].to_uppercase();

                    calls.push(ToolCall {
                        name: "create_edge".to_string(),
                        parameters: serde_json::json!({
                            "from": from,
                            "to": to
                        }),
                    });
                }
            }
        }
    }

    Ok(calls)
}

/// Extract JSON from markdown code blocks
fn extract_json_from_markdown(raw_str: &str) -> &str {
    let trimmed = raw_str.trim();

    // Check for ```json blocks
    if let Some(start) = trimmed.find("```json") {
        let remainder = &trimmed[start + 7..];
        if let Some(end) = remainder.find("```") {
            return remainder[..end].trim();
        }
    }

    // Check for ``` blocks
    if let Some(start) = trimmed.find("```") {
        let remainder = &trimmed[start + 3..];
        if let Some(end) = remainder.find("```") {
            return remainder[..end].trim();
        }
    }

    // Look for first { to last }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return trimmed[start..=end].trim();
            }
        }
    }

    trimmed
}
