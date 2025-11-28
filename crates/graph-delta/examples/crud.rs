use graph_delta::{
    commands::{DotCommand, apply_command},
    parser::{chunks_to_complete_dot, parse_dot_to_chunks},
};

// Example: examples/crud_operations.rs
// Run with: cargo run --example crud_operations
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
    let cmd = DotCommand::CreateNode {
        id: "HTMLNode".to_string(),
        attrs: Some("shape=plaintext label=<<table><tr><td>HTML</td></tr></table>>".to_string()),
        parent: None,
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: CreateNode HTMLNode");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 2: Update existing node
    println!("=== Operation 2: Update node attributes ===");
    let cmd = DotCommand::UpdateNode {
        id: "A".to_string(),
        attrs: Some(
            "label=\"Modified A\" color=red fillcolor=\"#ffcccc\" style=filled".to_string(),
        ),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: UpdateNode A");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 3: Create edge with port
    println!("=== Operation 3: Create edge with port ===");
    let cmd = DotCommand::CreateEdge {
        from: "B".to_string(),
        to: "HTMLNode".to_string(),
        attrs: Some("label=\"to HTML\" color=blue penwidth=2".to_string()),
        parent: None,
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: CreateEdge B -> HTMLNode");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 4: Set graph-level attribute
    println!("=== Operation 4: Set graph attribute ===");
    let cmd = DotCommand::SetGraphAttr {
        key: "rankdir".to_string(),
        value: "LR".to_string(),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: SetGraphAttr rankdir=LR");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 5: Set node defaults
    println!("=== Operation 5: Set node defaults ===");
    let cmd = DotCommand::SetNodeDefault {
        attrs: "shape=box style=filled fillcolor=\"#e8f4ff\"".to_string(),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: SetNodeDefault");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 6: Create a subgraph
    println!("=== Operation 6: Create subgraph ===");
    let cmd = DotCommand::CreateSubgraph {
        id: Some("cluster_Main".to_string()),
        parent: None,
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: CreateSubgraph cluster_Main");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 6b: Add nodes INSIDE the subgraph
    println!("=== Operation 6b: Add nodes inside subgraph ===");
    let cmd = DotCommand::CreateNode {
        id: "InCluster1".to_string(),
        attrs: Some("label=\"Inside Cluster\" fillcolor=\"#ffffcc\"".to_string()),
        parent: Some("cluster_Main".to_string()),
    };
    apply_command(&mut chunks, &cmd).unwrap();

    let cmd = DotCommand::CreateNode {
        id: "InCluster2".to_string(),
        attrs: Some("label=\"Also Inside\" fillcolor=\"#ffffcc\"".to_string()),
        parent: Some("cluster_Main".to_string()),
    };
    apply_command(&mut chunks, &cmd).unwrap();

    let cmd = DotCommand::CreateEdge {
        from: "InCluster1".to_string(),
        to: "InCluster2".to_string(),
        attrs: Some("label=\"internal\"".to_string()),
        parent: Some("cluster_Main".to_string()),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: Created 2 nodes and 1 edge inside cluster_Main");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 7: Update edge
    println!("=== Operation 7: Update edge ===");
    let cmd = DotCommand::UpdateEdge {
        from: "A".to_string(),
        to: "B".to_string(),
        attrs: Some("label=\"updated\" color=green style=dashed".to_string()),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: UpdateEdge A -> B");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 8: Delete node (and show error handling)
    println!("=== Operation 8: Delete node ===");
    let cmd = DotCommand::DeleteNode {
        id: "HTMLNode".to_string(),
    };
    apply_command(&mut chunks, &cmd).unwrap();
    println!("Command: DeleteNode HTMLNode");
    println!("Result:");
    println!("{}\n", chunks_to_complete_dot(&chunks, Some("Example")));

    // Example 9: Try to delete non-existent node (error case)
    println!("=== Operation 9: Error handling - delete non-existent node ===");
    let cmd = DotCommand::DeleteNode {
        id: "NonExistent".to_string(),
    };
    match apply_command(&mut chunks, &cmd) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}\n", e),
    }

    // Example 10: Complex workflow - build a small network
    println!("=== Operation 10: Complex workflow ===");
    let operations = vec![
        DotCommand::CreateNode {
            id: "Server".to_string(),
            attrs: Some("label=\"Web Server\" shape=box3d fillcolor=\"#ccffcc\"".to_string()),
            parent: None,
        },
        DotCommand::CreateNode {
            id: "DB".to_string(),
            attrs: Some("label=\"Database\" shape=cylinder fillcolor=\"#ccccff\"".to_string()),
            parent: None,
        },
        DotCommand::CreateNode {
            id: "Cache".to_string(),
            attrs: Some("label=\"Cache\" shape=component fillcolor=\"#ffcccc\"".to_string()),
            parent: None,
        },
        DotCommand::CreateEdge {
            from: "Server".to_string(),
            to: "DB".to_string(),
            attrs: Some("label=\"query\"".to_string()),
            parent: None,
        },
        DotCommand::CreateEdge {
            from: "Server".to_string(),
            to: "Cache".to_string(),
            attrs: Some("label=\"read/write\" style=dashed".to_string()),
            parent: None,
        },
        DotCommand::CreateEdge {
            from: "Cache".to_string(),
            to: "DB".to_string(),
            attrs: Some("label=\"miss\" color=red".to_string()),
            parent: None,
        },
    ];

    for c in &operations {
        apply_command(&mut chunks, c).unwrap();
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
