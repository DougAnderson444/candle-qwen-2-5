//! A graph library with named nodes built on petgraph's StableGraph.
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences as _};
use petgraph::{Directed, EdgeType, Graph, Undirected};
use std::collections::HashMap;
use std::fmt::Display;

/// Convert any StableGraph<N, E, Ty> into a StableGraph<String, NewE, Ty>.
/// The caller provides:
///   - `extract_name`: map &N -> String (how to get a node name)
///   - `map_edge`: map &E -> NewE (how to convert edge weights)
pub fn convert_nodes_and_map_edges<N, E, Ty, F, G, NewE>(
    g: StableGraph<N, E, Ty>,
    mut extract_name: F,
    mut map_edge: G,
) -> StableGraph<String, NewE, Ty>
where
    F: FnMut(&N) -> String,
    G: FnMut(&E) -> NewE,
    Ty: EdgeType,
{
    let mut out = StableGraph::with_capacity(g.node_count(), g.edge_count());
    let mut map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    for ni in g.node_indices() {
        let name = extract_name(g.node_weight(ni).unwrap());
        let new_ni = out.add_node(name);
        map.insert(ni, new_ni);
    }

    for e in g.edge_references() {
        let s = e.source();
        let t = e.target();
        let new_w = map_edge(e.weight());
        out.add_edge(map[&s], map[&t], new_w);
    }

    out
}

/// NamedGraph owns String node weights (so all mutation APIs are easy).
pub struct NamedGraph<E, Ty: EdgeType = Undirected> {
    graph: StableGraph<String, E, Ty>,
    name_map: HashMap<String, NodeIndex>,
    node_to_subgraph: HashMap<String, String>, // node name -> subgraph name
}

impl<E> NamedGraph<E, Undirected> {
    pub fn new_undirected() -> Self {
        Self {
            graph: Graph::new_undirected().into(),
            name_map: HashMap::new(),
            node_to_subgraph: HashMap::new(),
        }
    }
}

impl<E> NamedGraph<E, Directed> {
    pub fn new_directed() -> Self {
        Self {
            graph: StableGraph::new(),
            name_map: HashMap::new(),
            node_to_subgraph: HashMap::new(),
        }
    }
}

impl<E, Ty> NamedGraph<E, Ty>
where
    Ty: EdgeType,
{
    pub fn from_owned_graph(graph: StableGraph<String, E, Ty>) -> Self {
        let mut name_map = HashMap::new();
        for idx in graph.node_indices() {
            if let Some(name) = graph.node_weight(idx) {
                name_map.insert(name.clone(), idx);
            }
        }
        Self {
            graph,
            name_map,
            node_to_subgraph: HashMap::new(),
        }
    }

    pub fn graph(&self) -> &StableGraph<String, E, Ty> {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut StableGraph<String, E, Ty> {
        &mut self.graph
    }

    pub fn get_node_index(&self, name: &str) -> Option<NodeIndex> {
        self.name_map.get(name).copied()
    }

    pub fn ensure_node(&mut self, name: impl Into<String>) -> NodeIndex {
        let name_owned = name.into();
        if let Some(&idx) = self.name_map.get(&name_owned) {
            return idx;
        }
        let idx = self.graph.add_node(name_owned.clone());
        self.name_map.insert(name_owned, idx);
        idx
    }

    pub fn add_edge_by_name(&mut self, a: &str, b: &str, weight: E) -> petgraph::graph::EdgeIndex {
        let ia = self.ensure_node(a.to_string());
        let ib = self.ensure_node(b.to_string());
        self.graph.add_edge(ia, ib, weight)
    }

    pub fn remove_node_by_name(&mut self, name: &str) -> Option<String> {
        let idx = self.name_map.remove(name)?;
        self.graph.remove_node(idx)
    }

    pub fn remove_edge_by_names(&mut self, a: &str, b: &str) -> Option<E> {
        let ia = self.get_node_index(a)?;
        let ib = self.get_node_index(b)?;
        if let Some(ei) = self.graph.find_edge(ia, ib) {
            self.graph.remove_edge(ei)
        } else {
            None
        }
    }

    pub fn neighbors_by_name(&self, name: &str) -> Option<Vec<String>> {
        let idx = self.get_node_index(name)?;
        let mut res = Vec::new();
        for n in self.graph.neighbors(idx) {
            if let Some(w) = self.graph.node_weight(n) {
                res.push(w.clone());
            }
        }
        Some(res)
    }

    pub fn node_names(&self) -> Vec<String> {
        self.graph.node_weights().cloned().collect::<Vec<_>>()
    }

    pub fn edges_with_names(&self) -> Vec<(String, String, E)>
    where
        E: Clone,
    {
        let mut out = Vec::new();
        for e in self.graph.edge_references() {
            let s = self.graph.node_weight(e.source()).unwrap().clone();
            let t = self.graph.node_weight(e.target()).unwrap().clone();
            out.push((s, t, e.weight().clone()));
        }
        out
    }

    pub fn rename_node(&mut self, old_name: &str, new_name: impl Into<String>) -> bool {
        let new_name = new_name.into();
        if self.name_map.contains_key(&new_name) {
            return false;
        }
        let idx = match self.name_map.remove(old_name) {
            Some(idx) => idx,
            None => return false,
        };
        if let Some(w) = self.graph.node_weight_mut(idx) {
            *w = new_name.clone();
            self.name_map.insert(new_name, idx);
            true
        } else {
            false
        }
    }

    pub fn set_node_subgraph(&mut self, node_name: &str, subgraph_name: impl Into<String>) {
        if self.name_map.contains_key(node_name) {
            self.node_to_subgraph
                .insert(node_name.to_string(), subgraph_name.into());
        }
    }

    pub fn to_dot(&self) -> String
    where
        E: Clone + Display,
        (String, String, E): Ord,
    {
        let mut dot_output = String::new();
        let graph_type = if self.graph.is_directed() {
            "digraph"
        } else {
            "graph"
        };
        let edge_op = if self.graph.is_directed() { "->" } else { "--" };

        dot_output.push_str(&format!("{} G {{\n", graph_type));

        let mut subgraph_nodes: HashMap<String, Vec<String>> = HashMap::new();
        let mut root_nodes: Vec<String> = Vec::new();

        for node_name in self.graph.node_weights().cloned() {
            if let Some(subgraph_name) = self.node_to_subgraph.get(&node_name) {
                subgraph_nodes
                    .entry(subgraph_name.clone())
                    .or_default()
                    .push(node_name);
            } else {
                root_nodes.push(node_name);
            }
        }
        root_nodes.sort();

        let mut subgraph_keys: Vec<_> = subgraph_nodes.keys().cloned().collect();
        subgraph_keys.sort();

        for (i, subgraph_name) in subgraph_keys.iter().enumerate() {
            dot_output.push_str(&format!("    subgraph cluster_{} {{\n", i));
            dot_output.push_str(&format!("        label = \"{}\";\n", subgraph_name));
            if let Some(nodes) = subgraph_nodes.get(subgraph_name) {
                let mut sorted_nodes = nodes.clone();
                sorted_nodes.sort();
                for node_name in &sorted_nodes {
                    dot_output.push_str(&format!("        \"{}\";\n", node_name));
                }
            }
            dot_output.push_str("    }\n");
        }

        for node_name in &root_nodes {
            dot_output.push_str(&format!("    \"{}\";\n", node_name));
        }

        let mut sorted_edges = self.edges_with_names();
        sorted_edges.sort();

        for (s, t, w) in &sorted_edges {
            let edge_label = w.to_string();
            let label_attr = if edge_label.trim().starts_with('<') && edge_label.trim().ends_with('>')
            {
                format!("label={}", edge_label)
            } else {
                format!("label=\"{}\"", edge_label)
            };

            dot_output.push_str(&format!(
                "    \"{}\" {} \"{}\" [{}];\n",
                s, edge_op, t, label_attr
            ));
        }

        dot_output.push_str("}\n");
        dot_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::dot::dot_parser::{DotNodeWeight, ParseFromDot};
    use petgraph::stable_graph::StableGraph;

    #[test]
    fn basic_ops() {
        let mut ng = NamedGraph::<i32>::new_undirected();
        ng.add_edge_by_name("A", "B", 1);
        ng.add_edge_by_name("B", "C", 2);
        ng.add_edge_by_name("A", "C", 3);

        let nb = ng.neighbors_by_name("A").unwrap();
        assert!(nb.contains(&"B".to_string()));
        assert!(nb.contains(&"C".to_string()));

        assert!(ng.remove_edge_by_names("A", "C").is_some());
        let nb_after = ng.neighbors_by_name("A").unwrap();
        assert!(!nb_after.contains(&"C".to_string()));

        assert!(ng.remove_node_by_name("B").is_some());
        assert!(ng.get_node_index("B").is_none());
    }

    #[test]
    fn parse_digraph_and_modify() {
        let dot = r#"digraph { "Alice" -> "Bob"; }"#;

        // Use the official `ParseFromDot` trait. This creates a graph with petgraph's
        // internal dot parser types as weights.
        let s_graph: StableGraph<DotNodeWeight, _> = ParseFromDot::try_from(dot).unwrap();

        // The extractor now correctly accesses the `id` field and trims the quotes.
        let extractor = |n: &DotNodeWeight| n.id.to_string().trim_matches('"').to_string();

        // Convert the parsed graph into a graph with simple `String` node weights.
        let owned: StableGraph<String, i32, _> =
            convert_nodes_and_map_edges(s_graph, extractor, |_| 1);

        let mut ng = NamedGraph::from_owned_graph(owned);

        // Check initial state
        let initial_edges = ng.edges_with_names();
        assert_eq!(initial_edges.len(), 1);
        assert_eq!(
            initial_edges[0],
            ("Alice".to_string(), "Bob".to_string(), 1)
        );

        // Now, modify the graph
        ng.add_edge_by_name("Bob", "Carol", 1);

        let mut final_edges = ng.edges_with_names();
        final_edges.sort(); // Sort for deterministic order in asserts and output

        assert_eq!(final_edges.len(), 2);
        assert_eq!(final_edges[0], ("Alice".to_string(), "Bob".to_string(), 1));
        assert_eq!(final_edges[1], ("Bob".to_string(), "Carol".to_string(), 1));

        let dot_output = ng.to_dot();

        let expected_dot = r#"digraph G {
    "Alice";
    "Bob";
    "Carol";
    "Alice" -> "Bob" [label="1"];
    "Bob" -> "Carol" [label="1"];
}
"#;

        assert_eq!(dot_output, expected_dot);
    }

    #[test]
    fn add_subgraph() {
        let mut ng = NamedGraph::<i32, Directed>::new_directed();
        ng.add_edge_by_name("A", "B", 1);
        ng.add_edge_by_name("C", "D", 1);
        ng.add_edge_by_name("A", "C", 2);

        // Assign nodes to subgraphs
        ng.set_node_subgraph("A", "Subgraph 1");
        ng.set_node_subgraph("B", "Subgraph 1");
        ng.set_node_subgraph("C", "Subgraph 2");
        ng.set_node_subgraph("D", "Subgraph 2");

        let dot_output = ng.to_dot();

        // The to_dot method sorts keys to ensure deterministic output
        let expected_dot = r#"digraph G {
    subgraph cluster_0 {
        label = "Subgraph 1";
        "A";
        "B";
    }
    subgraph cluster_1 {
        label = "Subgraph 2";
        "C";
        "D";
    }
    "A" -> "B" [label="1"];
    "A" -> "C" [label="2"];
    "C" -> "D" [label="1"];
}
"#;
        assert_eq!(dot_output, expected_dot);
    }

    #[test]
    fn parse_with_edge_label() {
        let dot = r#"digraph { A -> B [label = "MyLabel"]; }"#;

        let s_graph: StableGraph<DotNodeWeight, _> = ParseFromDot::try_from(dot).unwrap();

        let extractor = |n: &DotNodeWeight| n.id.to_string().trim_matches('"').to_string();
        let edge_mapper = |attrs: &petgraph::dot::dot_parser::DotAttrList| {
            if let Some(attr) = attrs.elems.iter().find(|(k, _)| k == &"label") {
                attr.1.to_string().trim_matches('"').to_string()
            } else {
                String::new()
            }
        };

        let owned: StableGraph<String, String, _> =
            convert_nodes_and_map_edges(s_graph, extractor, edge_mapper);

        let ng = NamedGraph::<String, Directed>::from_owned_graph(owned);

        let edges = ng.edges_with_names();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].2, "MyLabel");

        let dot_output = ng.to_dot();
        assert!(dot_output.contains(r#"[label="MyLabel"]"#));
    }

    #[test]
    fn parse_with_html_edge_label() {
        let dot = r#"digraph { A -> B [label=<simple text>]; }"#;

        let s_graph: StableGraph<DotNodeWeight, _> = ParseFromDot::try_from(dot).unwrap();

        let extractor = |n: &DotNodeWeight| n.id.to_string().trim_matches('"').to_string();
        let edge_mapper = |attrs: &petgraph::dot::dot_parser::DotAttrList| {
            if let Some(attr) = attrs.elems.iter().find(|(k, _)| k == &"label") {
                attr.1.to_string()
            } else {
                String::new()
            }
        };

        let owned: StableGraph<String, String, _> =
            convert_nodes_and_map_edges(s_graph, extractor, edge_mapper);

        let ng = NamedGraph::<String, Directed>::from_owned_graph(owned);
        let edges = ng.edges_with_names();
        let html_label = &edges[0].2;

        assert_eq!(html_label, "<simple text>");

        let dot_output = ng.to_dot();
        // Check that the output is label=<...> and not label="..."
        assert!(dot_output.contains("[label=<simple text>]"));
    }
}
