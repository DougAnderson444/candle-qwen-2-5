//! An example of using `graph-delta` with an LLM to edit a DOT file.
//!
//! This example performs the following steps:
//! 1. Reads an initial DOT graph from a file.
//! 2. Reads a base LLM prompt.
//! 3. Constructs a full prompt including a user request to modify the graph.
//! 4. Uses `candle-qwen2-5-core` to estimate the number of tokens in the prompt.
//! 5. Prints the token count and the time taken for the estimation.
//! 6. Asks the LLM to generate `DotCommand` JSON based on the prompt.
//! 7. Parses the JSON response from the LLM.
//! 8. Applies the parsed commands to the graph.
//! 9. Prints the final, modified DOT graph to the console.
//!
//! ## Prerequisites
//!
//! This example requires the `llm` feature to be enabled, which includes the `candle-qwen2-5-core` dependency.
//!
//! ## Usage
//!
//! Run this example from the root of the workspace using the following command:
//!
//! ```sh
//! cargo run --release --example llm_editor --features graph-delta/llm
//! ```
//! Note: The first run will download the Qwen2 model files, which may take some time.

use anyhow::Result;
use serde_json;
use std::io::Write;
use std::time::Instant;

use graph_delta::{
    commands::{DotCommand, apply_command},
    parser::{chunks_to_complete_dot, parse_dot_to_chunks},
};

use candle_qwen2_5_core::{ModelArgs, Qwen2Model};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== LLM DOT File Editor Example ===");

    let initial_dot = include_str!("../tests/fixtures/simple_example.dot");
    let llm_prompt_template = include_str!("../generated/llm_prompt.txt");

    // 1. Define the change instruction and construct the full prompt
    let user_instruction = "Add a new node C and connect it to node A";
    let full_prompt = format!(
        "{}\n\nCURRENT_GRAPH:\n```dot\n{}\n```\n\nINSTRUCTION:\n{}",
        llm_prompt_template, initial_dot, user_instruction
    );



    println!("\n--- Initializing Model & Estimating Prompt Tokens ---");

    // 2. Initialize the model
    let model_args = ModelArgs::default();
    let mut model = Qwen2Model::new(&model_args).await?;

    // 3. Estimate tokens
    let start_time = Instant::now();
    let token_count = model.estimate_prompt_tokens(&full_prompt)?;
    let duration = start_time.elapsed();

    println!("Prompt token count: {}", token_count);
    println!("Time to estimate tokens: {:?}", duration);

    // 4. Generate commands from the LLM
    println!("\n--- Generating Commands from LLM ---");
    let mut llm_output = String::new();
    let start_generation_time = Instant::now();

    let sample_len = 1024; // Max tokens to generate for the commands

    print!("\nLLM Output Stream: ");
    std::io::stdout().flush()?;

    model.generate(&full_prompt, sample_len, |s| {
        print!("{}", s);
        std::io::stdout().flush()?;
        llm_output.push_str(&s);
        Ok(())
    })?;

    let generation_duration = start_generation_time.elapsed();
    println!(); // Newline after stream
    println!("\nTime to generate commands: {:?}", generation_duration);


    // 5. Parse the LLM's JSON response
    println!("\n--- Parsing LLM-generated Commands ---");
    // The LLM might wrap the JSON in a code block, so we extract it.
    let json_str = if let Some(start) = llm_output.find("```json") {
        let remainder = &llm_output[start + 7..];
        if let Some(end) = remainder.find("```") {
            &remainder[..end].trim()
        } else {
            remainder.trim()
        }
    } else {
        llm_output.trim()
    };

    // The LLM can return a single command object or an array of them.
    let commands: Vec<DotCommand> = if let Ok(cmds) = serde_json::from_str(json_str) {
        cmds
    } else {
        match serde_json::from_str::<DotCommand>(json_str) {
            Ok(cmd) => vec![cmd],
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to parse LLM output as JSON array or object: {}. Raw output was: {}",
                    e,
                    json_str
                ));
            }
        }
    };
    println!("Successfully parsed {} command(s).", commands.len());

    // 6. Parse the initial DOT file and apply changes
    let mut chunks = parse_dot_to_chunks(initial_dot).expect("Failed to parse initial DOT");

    println!("\n--- Applying Changes ---");
    println!("Applying {} commands...", commands.len());
    for cmd in &commands {
        println!("- {}", cmd);
        apply_command(&mut chunks, cmd).map_err(|e| anyhow::anyhow!(e))?;
    }

    // 7. Print the modified DOT graph
    let modified_dot = chunks_to_complete_dot(&chunks, Some("G"));

    println!("\n--- Modified DOT Graph ---");
    println!("{}", modified_dot);

    Ok(())
}
