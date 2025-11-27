// // Parse DOT file
// let chunks = parse_dot_to_chunks(&dot_string)?;
//
// // Modify a chunk (e.g., change node color)
// for chunk in &mut chunks {
//     if chunk.id.as_deref() == Some("A1") {
//         chunk.attrs = Some("color=\"red\" label=\"Modified A1\"".to_string());
//     }
// }
//
// // Reconstruct DOT
// let new_dot = chunks_to_dot(&chunks, Some("MyGraph"));
use graph_delta::parser::{chunks_to_complete_dot, parse_dot_to_chunks};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dot_string = r#"
    digraph MyGraph {
        A1 [color="blue" label="Node A1"];
        B1 [color="green" label="Node B1"];
        A1 -> B1 [label="Edge from A1 to B1"];
    }
    "#;

    // Parse DOT file
    let mut chunks = parse_dot_to_chunks(dot_string)?;

    // Modify a chunk (e.g., change node color)
    for chunk in &mut chunks {
        if chunk.id.as_deref() == Some("A1") {
            chunk.attrs = Some("color=\"red\" label=\"Modified A1\"".to_string());
        }
    }

    // Reconstruct DOT
    let new_dot = chunks_to_complete_dot(&chunks, Some("MyGraph"));
    println!("{}", new_dot);

    Ok(())
}
