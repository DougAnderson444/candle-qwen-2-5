use graph_delta::parser::parse_dot_to_chunks;

fn main() {
    // This works (from crud example)
    let dot1 = r#"digraph Example {
    A [label="Node A"];
    B [label="Node B"];
    A -> B [label="edge"];
}"#;
    
    println!("Test 1 - Basic quoted attributes:");
    match parse_dot_to_chunks(dot1) {
        Ok(chunks) => {
            println!("  ✓ Success! {} chunks", chunks.len());
            for chunk in &chunks {
                println!("    {:?}", chunk);
            }
        }
        Err(e) => println!("  ✗ Error: {}", e),
    }
    
    // This should also work
    let dot2 = r#"digraph Example {
    A [label="Node A" color=blue];
}"#;
    
    println!("\nTest 2 - Mixed quoted and unquoted:");
    match parse_dot_to_chunks(dot2) {
        Ok(chunks) => {
            println!("  ✓ Success! {} chunks", chunks.len());
            for chunk in &chunks {
                println!("    {:?}", chunk);
            }
        }
        Err(e) => println!("  ✗ Error: {}", e),
    }
    
    // Try with all quoted
    let dot3 = r#"digraph Example {
    A [label="Node A", color="blue"];
}"#;
    
    println!("\nTest 3 - All quoted with comma:");
    match parse_dot_to_chunks(dot3) {
        Ok(chunks) => {
            println!("  ✓ Success! {} chunks", chunks.len());
            for chunk in &chunks {
                println!("    {:?}", chunk);
            }
        }
        Err(e) => println!("  ✗ Error: {}", e),
    }
    
    // Try exact dsl_editor format
    let dot4 = r#"
    digraph G {
        A [label="Node A", color="blue"];
        B [label="Node B", shape="box", fontsize="10"];
        A -> B [label="Original Edge", penwidth="2"];
    }
    "#;
    
    println!("\nTest 4 - Exact dsl_editor format:");
    match parse_dot_to_chunks(dot4) {
        Ok(chunks) => {
            println!("  ✓ Success! {} chunks", chunks.len());
            for chunk in &chunks {
                println!("    {:?}", chunk);
            }
        }
        Err(e) => println!("  ✗ Error: {}", e),
    }
}
