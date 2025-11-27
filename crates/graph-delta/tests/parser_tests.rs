use graph_delta::parser::parse_dot_to_chunks;

#[test]
fn test_parse_dot_to_chunks_basic() {
    let dot = r#"
        digraph G {
            A [label="Node A"];
            B [label="Node B"];
            A -> B [label="Edge from A to B"];
            subgraph cluster_0 {
                C [label="Node C"];
            }
        }
    "#;

    let chunks = parse_dot_to_chunks(dot);

    assert!(
        chunks
            .iter()
            .any(|c| c.kind == "node" && c.id.as_deref() == Some("A"))
    );
    assert!(
        chunks
            .iter()
            .any(|c| c.kind == "node" && c.id.as_deref() == Some("B"))
    );
    assert!(
        chunks
            .iter()
            .any(|c| c.kind == "edge" && c.id.as_deref() == Some("A"))
    );
    assert!(chunks.iter().any(|c| c.kind == "subgraph"));
}

#[test]
fn test_parse_dot_to_chunks_kitchen_sink() {
    let dot = std::fs::read_to_string("./tests/fixtures/kitchen_sink.dot")
        .expect("Failed to read kitchen_sink.dot");

    let chunks = parse_dot_to_chunks(&dot);

    // Expect at least 10 chunks (nodes, edges, subgraphs, etc.)
    assert!(
        chunks.len() >= 10,
        "Expected at least 10 chunks, got {}",
        chunks.len()
    );

    // Check for some known node and edge IDs
    assert!(
        chunks
            .iter()
            .any(|c| c.kind == "node" && c.id.as_deref() == Some("A1")),
        "Missing node A1"
    );
    assert!(
        chunks
            .iter()
            .any(|c| c.kind == "edge" && c.id.as_deref() == Some("A1")),
        "Missing edge from A1"
    );
    assert!(
        chunks.iter().any(|c| c.kind == "subgraph"),
        "Missing subgraph"
    );
}
