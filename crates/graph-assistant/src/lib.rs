//! A graph library with named nodes built on petgraph's StableGraph.
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences as _};
use petgraph::{Directed, EdgeType, Graph, Undirected};
use std::collections::HashMap;

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
}

impl<E> NamedGraph<E, Undirected> {
    pub fn new_undirected() -> Self {
        Self {
            graph: Graph::new_undirected().into(),
            name_map: HashMap::new(),
        }
    }
}

impl<E> NamedGraph<E, Directed> {
    pub fn new_directed() -> Self {
        Self {
            graph: StableGraph::new(),
            name_map: HashMap::new(),
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
        Self { graph, name_map }
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

        let mut dot_output = String::new();
        dot_output.push_str("digraph {\n");
        for (s, t, w) in &final_edges {
            dot_output.push_str(&format!("    \"{}\" -> \"{}\" [weight={}];\n", s, t, w));
        }
        dot_output.push_str("}\n");

        let expected_dot = r#"digraph {
    "Alice" -> "Bob" [weight=1];
    "Bob" -> "Carol" [weight=1];
}
"#;

        assert_eq!(dot_output, expected_dot);
    }
}
