//! Run with:
//! ```sh
//! cargo run --bin generate_schema
//! ```
use graph_delta::commands::DotCommand;
use schemars::schema_for;
use std::fs;
use std::path::Path;

fn main() {
    println!("=== Generating DOT Command Schema ===\n");

    // Generate the JSON schema
    let schema = schema_for!(DotCommand);
    let schema_json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");

    // Create output directory
    let out_dir = Path::new("generated");
    fs::create_dir_all(out_dir).expect("Failed to create output directory");

    // Write schema file
    fs::write(out_dir.join("dot_command_schema.json"), &schema_json)
        .expect("Failed to write schema file");

    println!("✓ Generated: generated/dot_command_schema.json");

    // Generate examples
    let examples = generate_examples();
    let examples_md = format_examples_markdown(&examples);
    fs::write(out_dir.join("dot_command_examples.md"), &examples_md)
        .expect("Failed to write examples file");

    println!("✓ Generated: generated/dot_command_examples.md");

    // Generate LLM prompt
    let llm_prompt = generate_llm_prompt(&schema_json, &examples);
    fs::write(out_dir.join("llm_prompt.txt"), &llm_prompt)
        .expect("Failed to write LLM prompt file");

    println!("✓ Generated: generated/llm_prompt.txt\n");

    println!("=== Done! ===");
    println!("Run this script whenever you change DotCommand:");
    println!("  cargo run --bin generate_schema");
}

fn generate_examples() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "Create a node",
            serde_json::json!({
                "action": "create_node",
                "id": "NodeA",
                "attrs": "label=\"My Node\" shape=box fillcolor=\"#ccffcc\""
            }),
        ),
        (
            "Create a node inside a subgraph",
            serde_json::json!({
                "action": "create_node",
                "id": "NodeB",
                "attrs": "label=\"Inside Cluster\"",
                "parent": "cluster_Main"
            }),
        ),
        (
            "Update a node's attributes",
            serde_json::json!({
                "action": "update_node",
                "id": "NodeA",
                "attrs": "label=\"Updated\" color=red"
            }),
        ),
        (
            "Delete a node",
            serde_json::json!({
                "action": "delete_node",
                "id": "NodeA"
            }),
        ),
        (
            "Create an edge",
            serde_json::json!({
                "action": "create_edge",
                "from": "NodeA",
                "to": "NodeB",
                "attrs": "label=\"connects\" color=blue"
            }),
        ),
        (
            "Create an edge with port",
            serde_json::json!({
                "action": "create_edge",
                "from": "NodeA:p1",
                "to": "NodeB:p2",
                "attrs": "label=\"port connection\""
            }),
        ),
        (
            "Create an edge inside a subgraph",
            serde_json::json!({
                "action": "create_edge",
                "from": "NodeA",
                "to": "NodeB",
                "attrs": "label=\"internal\"",
                "parent": "cluster_Main"
            }),
        ),
        (
            "Update an edge",
            serde_json::json!({
                "action": "update_edge",
                "from": "NodeA",
                "to": "NodeB",
                "attrs": "label=\"modified\" style=dashed"
            }),
        ),
        (
            "Delete an edge",
            serde_json::json!({
                "action": "delete_edge",
                "from": "NodeA",
                "to": "NodeB"
            }),
        ),
        (
            "Create a subgraph/cluster",
            serde_json::json!({
                "action": "create_subgraph",
                "id": "cluster_Main"
            }),
        ),
        (
            "Create a nested subgraph",
            serde_json::json!({
                "action": "create_subgraph",
                "id": "cluster_Inner",
                "parent": "cluster_Main"
            }),
        ),
        (
            "Create anonymous subgraph (rank constraint)",
            serde_json::json!({
                "action": "create_subgraph"
            }),
        ),
        (
            "Delete a subgraph",
            serde_json::json!({
                "action": "delete_subgraph",
                "id": "cluster_Main"
            }),
        ),
        (
            "Set graph-level attribute",
            serde_json::json!({
                "action": "set_graph_attr",
                "key": "rankdir",
                "value": "LR"
            }),
        ),
        (
            "Set default node attributes",
            serde_json::json!({
                "action": "set_node_default",
                "attrs": "shape=box style=filled fillcolor=\"#e8f4ff\""
            }),
        ),
        (
            "Set default edge attributes",
            serde_json::json!({
                "action": "set_edge_default",
                "attrs": "color=\"#666666\" arrowsize=0.9"
            }),
        ),
        (
            "Delete a graph attribute",
            serde_json::json!({
                "action": "delete_attr",
                "key": "rankdir"
            }),
        ),
    ]
}

fn format_examples_markdown(examples: &[(&str, serde_json::Value)]) -> String {
    let mut output = String::new();
    output.push_str("# DOT Command Examples\n\n");
    output.push_str("These are valid JSON commands for modifying DOT graphs.\n");
    output.push_str("Generated automatically - do not edit manually.\n\n");

    for (description, command) in examples {
        output.push_str(&format!("## {}\n\n", description));
        output.push_str("```json\n");
        output.push_str(&serde_json::to_string_pretty(&command).unwrap());
        output.push_str("\n```\n\n");
    }

    output
}

fn generate_llm_prompt(schema: &str, examples: &[(&str, serde_json::Value)]) -> String {
    let mut prompt = String::new();

    prompt.push_str("# DOT Graph Manipulation Commands\n\n");
    prompt.push_str("You are helping users modify DOT graph files. ");
    prompt.push_str("Generate JSON commands that follow this schema:\n\n");
    prompt.push_str("```json\n");
    prompt.push_str(schema);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Available Commands\n\n");
    prompt.push_str("### Node Operations\n");
    prompt.push_str(
        "- `create_node`: Create a new node with optional attributes and parent subgraph\n",
    );
    prompt.push_str("  - `id`: Node identifier (required)\n");
    prompt.push_str(
        "  - `attrs`: Graphviz attributes like `label=\"...\" shape=box color=red` (optional)\n",
    );
    prompt.push_str("  - `parent`: Parent subgraph name to nest this node inside (optional)\n");
    prompt.push_str("- `update_node`: Update a node's attributes\n");
    prompt.push_str("  - `id`: Node identifier (required)\n");
    prompt.push_str("  - `attrs`: New Graphviz attributes (required)\n");
    prompt.push_str("- `delete_node`: Remove a node from the graph\n");
    prompt.push_str("  - `id`: Node identifier (required)\n\n");

    prompt.push_str("### Edge Operations\n");
    prompt.push_str("- `create_edge`: Create an edge between two nodes with optional attributes and parent subgraph\n");
    prompt.push_str("  - `from`: Source node ID, can include port like `NodeA:p1` (required)\n");
    prompt.push_str("  - `to`: Target node ID, can include port like `NodeB:p2` (required)\n");
    prompt.push_str("  - `attrs`: Graphviz attributes like `label=\"...\" color=blue style=dashed` (optional)\n");
    prompt.push_str("  - `parent`: Parent subgraph name to nest this edge inside (optional)\n");
    prompt.push_str("- `update_edge`: Update an edge's attributes\n");
    prompt.push_str("  - `from`: Source node ID (required)\n");
    prompt.push_str("  - `to`: Target node ID (required)\n");
    prompt.push_str("  - `attrs`: New Graphviz attributes (required)\n");
    prompt.push_str("- `delete_edge`: Remove an edge from the graph\n");
    prompt.push_str("  - `from`: Source node ID (required)\n");
    prompt.push_str("  - `to`: Target node ID (required)\n\n");

    prompt.push_str("### Subgraph Operations\n");
    prompt.push_str("- `create_subgraph`: Create a new subgraph/cluster with optional parent\n");
    prompt.push_str(
        "  - `id`: Subgraph identifier (use `cluster_` prefix for visible clusters) (optional)\n",
    );
    prompt.push_str("  - `parent`: Parent subgraph name for nesting (optional)\n");
    prompt.push_str("- `delete_subgraph`: Remove a subgraph and all its contents\n");
    prompt.push_str("  - `id`: Subgraph identifier (required)\n\n");

    prompt.push_str("### Attribute Operations\n");
    prompt.push_str("- `set_graph_attr`: Set a graph-level attribute (like rankdir, bgcolor)\n");
    prompt.push_str("  - `key`: Attribute name (required)\n");
    prompt.push_str("  - `value`: Attribute value (required)\n");
    prompt.push_str("- `set_node_default`: Set default attributes for all nodes\n");
    prompt.push_str("  - `attrs`: Default Graphviz attributes (required)\n");
    prompt.push_str("- `set_edge_default`: Set default attributes for all edges\n");
    prompt.push_str("  - `attrs`: Default Graphviz attributes (required)\n");
    prompt.push_str("- `delete_attr`: Remove a graph-level attribute\n");
    prompt.push_str("  - `key`: Attribute name (required)\n\n");

    prompt.push_str("## Command Examples\n\n");
    for (description, command) in examples.iter() {
        prompt.push_str(&format!("### {}\n", description));
        prompt.push_str("```json\n");
        prompt.push_str(&serde_json::to_string_pretty(&command).unwrap());
        prompt.push_str("\n```\n\n");
    }

    prompt.push_str("## Important Notes\n\n");
    prompt.push_str(
        "1. **Parent subgraphs**: Use the `parent` field to nest nodes/edges inside subgraphs\n",
    );
    prompt.push_str("2. **Cluster names**: Subgraph IDs starting with `cluster_` are rendered as visible clusters\n");
    prompt.push_str(
        "3. **Ports**: Node IDs can include ports like `NodeA:p1` for record-based nodes\n",
    );
    prompt.push_str("4. **HTML labels**: Attributes can include HTML-like labels using angle brackets: `label=<...>`\n");
    prompt.push_str(
        "5. **Attributes**: Use standard Graphviz attribute syntax in the `attrs` field\n",
    );
    prompt.push_str(
        "6. **Multiple commands**: Return a JSON array of commands for complex operations\n\n",
    );

    prompt.push_str("## Usage Pattern\n\n");
    prompt
        .push_str("When a user asks to modify a graph, respond with a JSON array of commands:\n\n");
    prompt.push_str("```json\n[\n");
    prompt.push_str(
        "  {\"action\": \"create_node\", \"id\": \"A\", \"attrs\": \"label=\\\"Node A\\\"\"},\n",
    );
    prompt.push_str(
        "  {\"action\": \"create_node\", \"id\": \"B\", \"attrs\": \"label=\\\"Node B\\\"\"},\n",
    );
    prompt.push_str("  {\"action\": \"create_edge\", \"from\": \"A\", \"to\": \"B\", \"attrs\": \"label=\\\"connects\\\"\"}\n");
    prompt.push_str("]\n```\n\n");

    prompt.push_str("## Common Patterns\n\n");
    prompt.push_str("**Creating a cluster with nodes:**\n");
    prompt.push_str("```json\n[\n");
    prompt.push_str("  {\"action\": \"create_subgraph\", \"id\": \"cluster_Main\"},\n");
    prompt.push_str("  {\"action\": \"create_node\", \"id\": \"N1\", \"parent\": \"cluster_Main\", \"attrs\": \"label=\\\"Node 1\\\"\"},\n");
    prompt.push_str("  {\"action\": \"create_node\", \"id\": \"N2\", \"parent\": \"cluster_Main\", \"attrs\": \"label=\\\"Node 2\\\"\"}\n");
    prompt.push_str("]\n```\n\n");

    prompt.push_str("**Styling nodes:**\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\"action\": \"update_node\", \"id\": \"N1\", \"attrs\": \"fillcolor=\\\"#ccffcc\\\" style=filled shape=box\"}\n");
    prompt.push_str("```\n\n");

    prompt.push_str("**Setting graph direction:**\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\"action\": \"set_graph_attr\", \"key\": \"rankdir\", \"value\": \"LR\"}\n");
    prompt.push_str("```\n");

    prompt
}
