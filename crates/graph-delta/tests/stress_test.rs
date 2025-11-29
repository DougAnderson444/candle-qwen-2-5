/// Stress tests for parser performance and stack safety
use graph_delta::dot_chunks::parser::parse_dot_to_chunks;

#[test]
#[ignore] // Run with: cargo test --test stress_test -- --ignored
fn test_large_statement_list() {
    // Test 10,000 simple node statements
    let mut dot = String::from("digraph G {\n");
    for i in 0..10_000 {
        dot.push_str(&format!("    node{} [label=\"Node {}\"];\n", i, i));
    }
    dot.push_str("}\n");

    let result = parse_dot_to_chunks(&dot);
    assert!(result.is_ok(), "Should parse 10k nodes without stack overflow");
    let chunks = result.unwrap();
    assert!(chunks.len() >= 10_000, "Should have at least 10k chunks");
}

#[test]
#[ignore]
fn test_long_edge_chain() {
    // Test chain of edges: A -> B -> C -> ... (potential recursion issue)
    let mut dot = String::from("digraph G {\n    ");
    
    // Create a chain of 1000 edges
    for i in 0..1000 {
        if i > 0 {
            dot.push_str(" -> ");
        }
        dot.push_str(&format!("node{}", i));
    }
    dot.push_str(";\n}\n");

    let result = parse_dot_to_chunks(&dot);
    assert!(result.is_ok(), "Should parse long edge chain without stack overflow");
}

#[test]
#[ignore]
fn test_many_chained_attributes() {
    // Test node with many chained attribute lists: node [a=1][b=2][c=3]...
    let mut dot = String::from("digraph G {\n    node1");
    
    // Create 100 chained attribute lists
    for i in 0..100 {
        dot.push_str(&format!(" [attr{}=\"value{}\"]", i, i));
    }
    dot.push_str(";\n}\n");

    let result = parse_dot_to_chunks(&dot);
    assert!(result.is_ok(), "Should parse many chained attributes without stack overflow");
}
