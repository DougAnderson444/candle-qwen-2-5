// Example: examples/crud_operations.rs
// Run with: cargo run --example crud_operations
use graph_delta::{
    commands::{DotCommand, apply_command},
    parser::{chunks_to_complete_dot, parse_dot_to_chunks},
};

fn main() {
    println!("=== DOT Graph CRUD Operations Example ===\n");

    // Start with a simple graph
    let initial_dot = r#"
digraph Example {
    A [label="Node A"];
    B [label="Node B"];
    A -> B [label="edge"];
}
"#;

    println!("Initial DOT:");
    println!("{}", initial_dot);

    // Parse to chunks
    let mut chunks = parse_dot_to_chunks(initial_dot).expect("Failed to parse initial DOT");

    println!("\nInitial chunks: {} items\n", chunks.len());

    // Example 1: Add a new node with HTML label
    println!("=== Operation 1: Add node with HTML label ===");
    let cmd_json = r#"{
        "action": "create_node",
        "id": "HTMLNode",
        "attrs": "shape=plaintext label=<<table><tr><td>HTML</td></tr></table>>"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 2: Update existing node
    println!("=== Operation 2: Update node attributes ===");
    let cmd_json = r#"{
        "action": "update_node",
        "id": "A",
        "attrs": "label=\"Modified A\" color=red fillcolor=\"\#ffcccc\" style=filled"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 3: Create edge with port
    println!("=== Operation 3: Create edge with port ===");
    let cmd_json = r#"{
        "action": "create_edge",
        "from": "B",
        "to": "HTMLNode",
        "attrs": "label=\"to HTML\" color=blue penwidth=2"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 4: Set graph-level attribute
    println!("=== Operation 4: Set graph attribute ===");
    let cmd_json = r#"{
        "action": "set_graph_attr",
        "key": "rankdir",
        "value": "LR"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 5: Set node defaults
    println!("=== Operation 5: Set node defaults ===");
    let cmd_json = r#"{
        "action": "set_node_default",
        "attrs": "shape=box style=filled fillcolor=\"\#e8f4ff\""
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 6: Create a subgraph
    println!("=== Operation 6: Create subgraph ===");
    let cmd_json = r#"{
        "action": "create_subgraph",
        "id": "cluster_Main"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 7: Update edge
    println!("=== Operation 7: Update edge ===");
    let cmd_json = r#"{
        "action": "update_edge",
        "from": "A",
        "to": "B",
        "attrs": "label=\"updated\" color=green style=dashed"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 8: Delete node (and show error handling)
    println!("=== Operation 8: Delete node ===");
    let cmd_json = r#"{
        "action": "delete_node",
        "id": "HTMLNode"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: {}", cmd_json);
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 9: Try to delete non-existent node (error case)
    println!("=== Operation 9: Error handling - delete non-existent node ===");
    let cmd_json = r#"{
        "action": "delete_node",
        "id": "NonExistent"
    }"#;

    let cmd: DotCommand = serde_json::from_str(cmd_json).unwrap();
    match apply_command(&mut chunks, &cmd) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}\n", e),
    }

    // Example 10: Complex workflow - build a small network
    println!("=== Operation 10: Complex workflow ===");
    let operations = vec![
        r#"{"action": "create_node", "id": "Server", "attrs": "label=\"Web Server\" shape=box3d fillcolor=\"\#ccffcc\""}"#,
        r#"{"action": "create_node", "id": "DB", "attrs": "label=\"Database\" shape=cylinder fillcolor=\"\#ccccff\""}"#,
        r#"{"action": "create_node", "id": "Cache", "attrs": "label=\"Cache\" shape=component fillcolor=\"\#ffcccc\""}"#,
        r#"{"action": "create_edge", "from": "Server", "to": "DB", "attrs": "label=\"query\""}"#,
        r#"{"action": "create_edge", "from": "Server", "to": "Cache", "attrs": "label=\"read/write\" style=dashed"}"#,
        r#"{"action": "create_edge", "from": "Cache", "to": "DB", "attrs": "label=\"miss\" color=red"}"#,
    ];

    for op_json in &operations {
        let cmd: DotCommand = serde_json::from_str(op_json).unwrap();
        apply_command(&mut chunks, &cmd).unwrap();
    }

    println!("Applied {} operations", operations.len());
    println!("Final graph:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    println!("=== Summary ===");
    println!("Total chunks: {}", chunks.len());
    println!(
        "Nodes: {}",
        chunks.iter().filter(|c| c.kind == "node").count()
    );
    println!(
        "Edges: {}",
        chunks.iter().filter(|c| c.kind == "edge").count()
    );
    println!(
        "Subgraphs: {}",
        chunks.iter().filter(|c| c.kind == "subgraph").count()
    );
}
