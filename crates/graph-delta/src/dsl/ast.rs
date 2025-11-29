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
    Add { id: String, attrs: Attrs },
    Update { id: String, attrs: Attrs },
    Delete { id: String },
}

#[derive(Debug)]
pub enum EdgeCmd {
    Add {
        from: String,
        to: String,
        attrs: Attrs,
    },
    Update {
        from: String,
        to: String,
        attrs: Attrs,
    },
    Delete {
        from: String,
        to: String,
    },
}

#[derive(Debug)]
pub enum ClusterCmd {
    Add { id: String, attrs: Attrs },
    Update { id: String, attrs: Attrs },
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
