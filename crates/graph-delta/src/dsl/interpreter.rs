//! Applies DslCommands to a vector of Chunks, modifying the graph structure.
use crate::dsl::ast::{ClusterCmd, DslCommand, EdgeCmd, GlobalCmd, NodeCmd, RankCmd};
use crate::parser::Chunk;

pub fn apply_commands(chunks: &mut Vec<Chunk>, cmds: Vec<DslCommand>) {
    for cmd in cmds {
        match cmd {
            DslCommand::Node(n) => apply_node(chunks, n),
            DslCommand::Edge(e) => apply_edge(chunks, e),
            DslCommand::Cluster(c) => apply_cluster(chunks, c),
            DslCommand::Global(g) => apply_global(chunks, g),
            DslCommand::Rank(r) => apply_rank(chunks, r),
        }
    }
}

/// Implementation for applying node commands to chunks
fn apply_node(chunks: &mut Vec<Chunk>, cmd: NodeCmd) {
    match cmd {
        NodeCmd::Add { id, attrs } => {
            chunks.push(Chunk {
                kind: "node".to_string(),
                id: Some(id),
                attrs,
                range: (0, 0), // New chunks have no original range
                extra: None,
            });
        }
        NodeCmd::Update { id, mut attrs } => {
            // First, handle rename if "id" is present in attributes.
            let new_id_opt = attrs.remove("id");
            let current_id = if let Some(new_id) = new_id_opt.clone() {
                // Update the node chunk itself
                if let Some(node_chunk) =
                    chunks.iter_mut().find(|c| c.kind == "node" && c.id.as_deref() == Some(&id))
                {
                    node_chunk.id = Some(new_id.clone());
                }
                // Update all edges connected to this node
                for edge_chunk in chunks.iter_mut().filter(|c| c.kind == "edge") {
                    if edge_chunk.id.as_deref() == Some(&id) {
                        edge_chunk.id = Some(new_id.clone());
                    }
                    if edge_chunk.extra.as_deref() == Some(&id) {
                        edge_chunk.extra = Some(new_id.clone());
                    }
                }
                // Update rank statements
                for rank_chunk in chunks.iter_mut().filter(|c| c.kind == "rank") {
                    if let Some(nodes_str) = rank_chunk.attrs.get_mut("nodes") {
                        *nodes_str = nodes_str
                            .split(',')
                            .map(|s| if s == id { new_id.clone() } else { s.to_string() })
                            .collect::<Vec<_>>()
                            .join(",");
                    }
                }
                new_id
            } else {
                id
            };

            // Update other attributes
            if !attrs.is_empty() {
                if let Some(node_chunk) = chunks
                    .iter_mut()
                    .find(|c| c.kind == "node" && c.id.as_deref() == Some(&current_id))
                {
                    node_chunk.attrs.extend(attrs);
                }
            }
        }
        NodeCmd::Delete { id } => {
            // Remove the node itself
            chunks.retain(|c| !(c.kind == "node" && c.id.as_deref() == Some(&id)));
            // Remove edges connected to the node
            chunks.retain(|c| {
                !(c.kind == "edge"
                    && (c.id.as_deref() == Some(&id) || c.extra.as_deref() == Some(&id)))
            });
        }
    }
}

/// Implementation for applying edge commands to chunks
fn apply_edge(chunks: &mut Vec<Chunk>, cmd: EdgeCmd) {
    match cmd {
        EdgeCmd::Add { from, to, attrs } => {
            chunks.push(Chunk {
                kind: "edge".to_string(),
                id: Some(from),
                extra: Some(to),
                attrs,
                range: (0, 0),
            });
        }
        EdgeCmd::Update { from, to, attrs } => {
            if let Some(edge_chunk) = chunks.iter_mut().find(|c| {
                c.kind == "edge"
                    && c.id.as_deref() == Some(&from)
                    && c.extra.as_deref() == Some(&to)
            }) {
                edge_chunk.attrs.extend(attrs);
            }
        }
        EdgeCmd::Delete { from, to } => {
            chunks.retain(|c| {
                !(c.kind == "edge"
                    && c.id.as_deref() == Some(&from)
                    && c.extra.as_deref() == Some(&to))
            });
        }
    }
}

/// Implementation for applying cluster/subgraph commands to chunks
fn apply_cluster(chunks: &mut Vec<Chunk>, cmd: ClusterCmd) {
    match cmd {
        ClusterCmd::Add { id, attrs } => {
            // Ensure cluster ID has "cluster_" prefix for dot layout engines
            let cluster_id = if id.starts_with("cluster_") {
                id
            } else {
                format!("cluster_{}", id)
            };
            chunks.push(Chunk {
                kind: "subgraph".to_string(),
                id: Some(cluster_id),
                attrs,
                range: (0, 0),
                extra: None,
            });
        }
        ClusterCmd::Update { id, attrs } => {
            let cluster_id = if id.starts_with("cluster_") {
                id
            } else {
                format!("cluster_{}", id)
            };
            if let Some(subgraph_chunk) = chunks
                .iter_mut()
                .find(|c| c.kind == "subgraph" && c.id.as_deref() == Some(&cluster_id))
            {
                subgraph_chunk.attrs.extend(attrs);
            }
        }
        ClusterCmd::Delete { id } => {
            let cluster_id = if id.starts_with("cluster_") {
                id
            } else {
                format!("cluster_{}", id)
            };
            // Note: This only removes the subgraph block. Nodes inside are NOT removed.
            chunks.retain(|c| !(c.kind == "subgraph" && c.id.as_deref() == Some(&cluster_id)));
        }
        ClusterCmd::Move { .. } => {
            // TODO: Implement node movement. This is a complex operation with the current
            // flat chunk structure, as it requires reordering chunks and potentially
            // adjusting line ranges to be represented correctly by `chunks_to_dot_nested`.
            // A more robust implementation would require a tree-like graph representation.
        }
    }
}

/// Implementation for applying global/default commands to chunks
fn apply_global(chunks: &mut Vec<Chunk>, cmd: GlobalCmd) {
    let (id, attrs_to_add) = match cmd {
        GlobalCmd::Set(attrs) => ("graph".to_string(), attrs),
        GlobalCmd::NodeDefaults(attrs) => ("node".to_string(), attrs),
        GlobalCmd::EdgeDefaults(attrs) => ("edge".to_string(), attrs),
    };

    if let Some(chunk) =
        chunks.iter_mut().find(|c| c.kind == "attr_stmt" && c.id.as_deref() == Some(&id))
    {
        chunk.attrs.extend(attrs_to_add);
    } else {
        chunks.push(Chunk {
            kind: "attr_stmt".to_string(),
            id: Some(id),
            attrs: attrs_to_add,
            range: (0, 0),
            extra: None,
        });
    }
}

/// Implementation for applying rank commands to chunks
fn apply_rank(chunks: &mut Vec<Chunk>, cmd: RankCmd) {
    let (kind, nodes) = match cmd {
        RankCmd::Same(nodes) => ("same", nodes),
        RankCmd::Min(nodes) => ("min", nodes),
        RankCmd::Max(nodes) => ("max", nodes),
    };

    let mut attrs = std::collections::HashMap::new();
    attrs.insert("nodes".to_string(), nodes.join(","));

    chunks.push(Chunk {
        kind: "rank".to_string(),
        id: Some(kind.to_string()),
        attrs,
        range: (0, 0),
        extra: None,
    });
}