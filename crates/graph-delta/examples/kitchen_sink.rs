//! An example demonstrating parsing and modifying a DOT graph using graph-delta.
//!
//! run with:
//! ```sh
//! cargo run --example kitchen_sink -- --nocapture
//! ```
use graph_delta::parser::{chunks_to_complete_dot, parse_dot_to_chunks};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dot_string = include_str!("../tests/fixtures/kitchen_sink.dot");

    // Parse DOT file
    let chunks = parse_dot_to_chunks(dot_string)?;

    // // Modify a chunk (e.g., change node color)
    // for chunk in &mut chunks {
    //     if chunk.id.as_deref() == Some("A1") {
    //         chunk.attrs = Some("color=\"red\" label=\"Modified A1\"".to_string());
    //     }
    // }

    // Reconstruct DOT
    let new_dot = chunks_to_complete_dot(&chunks, Some("KitchenSink"));
    println!("{}", new_dot);

    Ok(())
}
