use graph_assistant::NamedGraph;

fn main() {
    // Use undirected or directed explicitly:
    let mut ng = NamedGraph::<i32>::new_undirected();

    ng.add_edge_by_name("Alice", "Bob", 1);
    ng.add_edge_by_name("Bob", "Carol", 2);
    ng.add_edge_by_name("Alice", "Carol", 3);

    println!("Nodes: {:?}", ng.node_names());
    println!("Neighbors of Alice: {:?}", ng.neighbors_by_name("Alice"));

    ng.remove_edge_by_names("Alice", "Carol");
    println!(
        "Neighbors of Alice after removing edge: {:?}",
        ng.neighbors_by_name("Alice")
    );

    ng.remove_node_by_name("Bob");
    println!("Nodes after removing Bob: {:?}", ng.node_names());
}
