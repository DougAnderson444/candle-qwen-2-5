use std::collections::HashMap;

/// Data strucutres for parsed DSL
#[derive(Debug)]
pub enum DslCommand {
    Node(NodeCmd),
    Edge(EdgeCmd),
    Cluster(ClusterCmd),
    Global(GlobalCmd),
    Rank(RankCmd),
}

#[derive(Debug)]
pub enum NodeCmd {
    Set { id: String, attrs: Attrs },  // Auto-detects add vs update
    Delete { id: String },
}

#[derive(Debug)]
pub enum EdgeCmd {
    Set {
        from: String,
        to: String,
        attrs: Attrs,
    },  // Auto-detects add vs update
    Delete {
        from: String,
        to: String,
    },
}

#[derive(Debug)]
pub enum ClusterCmd {
    Set { id: String, attrs: Attrs },  // Auto-detects add vs update
    Delete { id: String },
    Move { node: String, cluster: String },
}

#[derive(Debug)]
pub enum GlobalCmd {
    Set(Attrs),
    NodeDefaults(Attrs),
    EdgeDefaults(Attrs),
}

#[derive(Debug)]
pub enum RankCmd {
    Same(Vec<String>),
    Min(Vec<String>),
    Max(Vec<String>),
}

pub type Attrs = HashMap<String, String>;
