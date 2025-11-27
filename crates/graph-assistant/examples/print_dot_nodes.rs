use petgraph::stable_graph::StableGraph;
use petgraph::dot::dot_parser::ParseFromDot;

fn main() {
    let dot = r#"digraph { "Alice" -> "Bob"; }"#;
    let s_graph: StableGraph<_, _> = ParseFromDot::try_from(dot).unwrap();

    // Print the raw node payloads (Debug) so we can craft an accurate extractor.
    eprintln!("raw node payloads: {:#?}", s_graph.node_weights().collect::<Vec<_>>());
}
