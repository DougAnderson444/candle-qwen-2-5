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
};

use candle_qwen2_5_core::{ModelArgs, Qwen2Model};

#[derive(Debug, Serialize, Deserialize)]
struct SimpleToolCall {
    tool: String,
    ids: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== LLM DOT File Editor Example (Simplified Agent) ===");

    // 1. Load and parse the graph into memory.
    let initial_dot = include_str!("../tests/fixtures/simple_example.dot");
    let mut chunks = parse_dot_to_chunks(initial_dot)
        .map_err(|e| anyhow::anyhow!("Failed to parse initial DOT graph: {}", e))?;

    // 2. Define the user's request.
    let user_instruction = "Add a new node C and connect it to node A";
    println!("\nUser Instruction: \"{}\"", user_instruction);

    // 3. Initialize the Model
    let model_args = ModelArgs {
        cpu: true, // Explicitly run on CPU for this example
        ..Default::default()
    };
    let mut model = Qwen2Model::new(&model_args).await?;

    // --- Agentic Workflow ---
    let start_time = Instant::now();

    // 4. LLM Call 1: Simple intent analysis to get node IDs.
    let prompt1_template = r#"# Task
You have one tool: `find_graph_nodes(node_ids: list[string])`.
Given a user request, identify all node IDs mentioned and respond with a JSON object to call this tool.
Format: `{{"tool": "find_graph_nodes", "ids": ["node_id_1", "node_id_2"]}}`
DO NOT add any other text, reasoning, or markdown.

# User Request
{user_instruction}

# Tool Call
"#;
    let prompt1 = prompt1_template.replace("{user_instruction}", user_instruction);

    println!("\n--- Step 1: Requesting Tool Call from LLM (Simple Format) ---");
    let mut llm_response1 = String::new();
    model.generate(&prompt1, 128, |s| {
        llm_response1.push_str(&s);
        Ok(())
    })?;
    println!("LLM Response (Tool Call): {}", llm_response1);

    // 5. Parse the simple tool call and execute in Rust.
    let clean_json1 = extract_json_from_markdown(&llm_response1);
    let tool_call: SimpleToolCall = serde_json::from_str(clean_json1)?;

    let node_ids_to_find = if tool_call.tool == "find_graph_nodes" {
        tool_call.ids
    } else {
        return Err(anyhow::anyhow!("LLM called an unknown tool."));
    };

    println!("\n--- Step 2: Executing Tool in Rust ---");
    println!("Searching for nodes: {:?}", node_ids_to_find);
    let search_results = find_graph_nodes(&node_ids_to_find, &chunks);
    println!("Tool Result: {}", search_results);

    // 6. LLM Call 2: Generate final command using context.
    let prompt2_template = r#"
    # Task
    You are a graph modification agent. Given a user request and context about existing nodes, generate the final `DotCommand` JSON to perform the action.
    Respond only with the final JSON command or array of commands.
    
    # Command Reference & JSON Format
    - `create_node(id, [attrs])` -> `{"action": "create_node", "id": "...", "attrs": "..."}`
    - `create_edge(from, to, [attrs])` -> `{"action": "create_edge", "from": "...", "to": "...", "attrs": "..."}`
    (The response should be a JSON array `[]` of one or more of these objects)
    
    # Context
    - User Request: "{user_instruction}"
    - Search Results: {search_results}
    
    # Final Command
    "#;
    let prompt2 = prompt2_template
        .replace("{user_instruction}", user_instruction)
        .replace("{search_results}", &search_results);

    println!("\n--- Step 3: Requesting Final Command from LLM ---");
    let mut llm_response2 = String::new();
    model.generate(&prompt2, 256, |s| {
        print!("{}", s);
        std::io::stdout().flush()?;
        llm_response2.push_str(&s);
        Ok(())
    })?;
    println!(); // Newline after stream

    // 7. Parse and apply the final command.
    println!("\n--- Step 4: Applying Final Command ---");
    let final_commands: Vec<DotCommand> = parse_final_command_json(&llm_response2)?;

    for cmd in &final_commands {
        println!("Applying command: {}", cmd);
        apply_command(&mut chunks, cmd)
            .map_err(|e| anyhow::anyhow!("Failed to apply command: {}", e))?;
    }

    // 8. Print the final state of the graph.
    let modified_dot = chunks_to_complete_dot(&chunks, Some("G"));
    println!("\n--- Final Modified Graph ---");
    println!("{}", modified_dot);
    println!("\nTotal execution time: {:?}", start_time.elapsed());

    Ok(())
}

/// Our "tool" implementation. It searches for nodes in the graph.
fn find_graph_nodes(node_ids: &[String], all_chunks: &[Chunk]) -> String {
    let mut found_nodes: Vec<&Chunk> = Vec::new();
    for id in node_ids {
        if let Some(chunk) = all_chunks
            .iter()
            .find(|c| c.kind == "node" && c.id.as_deref() == Some(id))
        {
            found_nodes.push(chunk);
        }
    }
    // Return a compact JSON string of the results.
    serde_json::to_string(&found_nodes).unwrap_or_else(|_| "[]".to_string())
}

/// Extracts a JSON object or array from a markdown code block or raw string.
fn extract_json_from_markdown(raw_str: &str) -> &str {
    let trimmed = raw_str.trim();
    if let Some(start) = trimmed.find("```json") {
        let remainder = &trimmed[start + 7..];
        if let Some(end) = remainder.rfind("```") {
            return remainder[..end].trim();
        }
        return remainder;
    }
    trimmed
}

/// Parses the LLM's final JSON output, which could be an array or a single object.
fn parse_final_command_json(json_str: &str) -> Result<Vec<DotCommand>> {
    let clean_json_str = extract_json_from_markdown(json_str);

    if let Ok(cmds) = serde_json::from_str(clean_json_str) {
        Ok(cmds)
    } else {
        match serde_json::from_str::<DotCommand>(clean_json_str) {
            Ok(cmd) => Ok(vec![cmd]),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to parse final command JSON: {}. Cleaned JSON was: {}",
                e,
                clean_json_str
            )),
        }
    }
}

