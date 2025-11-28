//! Commands for modifying DOT graph structures.
use crate::parser::Chunk;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DotCommand {
    // Node operations
    CreateNode {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<String>,
        /// Parent subgraph name, None = top level
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    UpdateNode {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<String>,
    },
    DeleteNode {
        id: String,
    },

    // Edge operations
    CreateEdge {
        from: String,
        to: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<String>,
        /// Parent subgraph name, None = top level
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
    },
    UpdateEdge {
        from: String,
        to: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attrs: Option<String>,
    },
    DeleteEdge {
        from: String,
        to: String,
    },

    // Subgraph operations
    CreateSubgraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<String>, // Parent subgraph name, None = top level
    },
    DeleteSubgraph {
        id: String,
    },

    // Attribute operations (for graph/node/edge defaults and id_eq statements)
    SetGraphAttr {
        key: String,
        value: String,
    },
    SetNodeDefault {
        attrs: String,
    },
    SetEdgeDefault {
        attrs: String,
    },
    DeleteAttr {
        key: String,
    },
}

impl std::fmt::Display for DotCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", json)
    }
}

pub fn apply_command(chunks: &mut Vec<Chunk>, command: &DotCommand) -> Result<(), String> {
    match command {
        DotCommand::CreateNode { id, attrs, parent } => {
            // Check if node already exists
            if chunks
                .iter()
                .any(|c| c.kind == "node" && c.id.as_ref() == Some(id))
            {
                return Err(format!("Node '{}' already exists", id));
            }

            // Find insertion point based on parent
            let (insert_pos, line) = if let Some(parent_name) = parent {
                // Find parent subgraph
                let parent_pos = chunks
                    .iter()
                    .position(|c| c.kind == "subgraph" && c.id.as_ref() == Some(parent_name))
                    .ok_or_else(|| format!("Parent subgraph '{}' not found", parent_name))?;

                let parent_range = chunks[parent_pos].range;

                // Find last item inside this parent (before parent's end line)
                let last_child_pos = chunks
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.range.0 > parent_range.0 && c.range.1 < parent_range.1)
                    .map(|(i, _)| i)
                    .max()
                    .unwrap_or(parent_pos);

                let line = if last_child_pos == parent_pos {
                    parent_range.0 + 1
                } else {
                    chunks[last_child_pos].range.1 + 1
                };

                (last_child_pos + 1, line)
            } else {
                // Insert after last top-level node
                let insert_pos = chunks
                    .iter()
                    .rposition(|c| c.kind == "node")
                    .map(|pos| pos + 1)
                    .unwrap_or(chunks.len());

                let line = if insert_pos > 0 {
                    chunks[insert_pos - 1].range.1 + 1
                } else {
                    1
                };

                (insert_pos, line)
            };

            chunks.insert(
                insert_pos,
                Chunk {
                    kind: "node".to_string(),
                    id: Some(id.clone()),
                    attrs: attrs.clone(),
                    range: (line, line),
                    extra: None,
                },
            );

            Ok(())
        }

        DotCommand::UpdateNode { id, attrs } => {
            let node = chunks
                .iter_mut()
                .find(|c| c.kind == "node" && c.id.as_ref() == Some(id))
                .ok_or_else(|| format!("Node '{}' not found", id))?;

            if let Some(new_attrs) = attrs {
                node.attrs = Some(new_attrs.clone());
            }
            Ok(())
        }

        DotCommand::DeleteNode { id } => {
            let pos = chunks
                .iter()
                .position(|c| c.kind == "node" && c.id.as_ref() == Some(id))
                .ok_or_else(|| format!("Node '{}' not found", id))?;

            chunks.remove(pos);
            Ok(())
        }

        DotCommand::CreateEdge {
            from,
            to,
            attrs,
            parent,
        } => {
            // Check if edge already exists
            if chunks.iter().any(|c| {
                c.kind == "edge" && c.id.as_ref() == Some(from) && c.extra.as_ref() == Some(to)
            }) {
                return Err(format!("Edge '{}' -> '{}' already exists", from, to));
            }

            // Find insertion point based on parent
            let (insert_pos, line) = if let Some(parent_name) = parent {
                // Find parent subgraph
                let parent_pos = chunks
                    .iter()
                    .position(|c| c.kind == "subgraph" && c.id.as_ref() == Some(parent_name))
                    .ok_or_else(|| format!("Parent subgraph '{}' not found", parent_name))?;

                let parent_range = chunks[parent_pos].range;

                // Find last item inside this parent
                let last_child_pos = chunks
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.range.0 > parent_range.0 && c.range.1 < parent_range.1)
                    .map(|(i, _)| i)
                    .max()
                    .unwrap_or(parent_pos);

                let line = if last_child_pos == parent_pos {
                    parent_range.0 + 1
                } else {
                    chunks[last_child_pos].range.1 + 1
                };

                (last_child_pos + 1, line)
            } else {
                // Insert after last top-level edge
                let insert_pos = chunks
                    .iter()
                    .rposition(|c| c.kind == "edge")
                    .map(|pos| pos + 1)
                    .unwrap_or(chunks.len());

                let line = if insert_pos > 0 {
                    chunks[insert_pos - 1].range.1 + 1
                } else {
                    1
                };

                (insert_pos, line)
            };

            chunks.insert(
                insert_pos,
                Chunk {
                    kind: "edge".to_string(),
                    id: Some(from.clone()),
                    attrs: attrs.clone(),
                    range: (line, line),
                    extra: Some(to.clone()),
                },
            );

            Ok(())
        }

        // DotCommand::UpdateEdge { from, to, attrs } => {
        //     let edge = chunks
        //         .iter_mut()
        //         .find(|c| {
        //             c.kind == "edge" && c.id.as_ref() == Some(from) && c.extra.as_ref() == Some(to)
        //         })
        //         .ok_or_else(|| format!("Edge '{}' -> '{}' not found", from, to))?;
        //
        //     if let Some(new_attrs) = attrs {
        //         edge.attrs = Some(new_attrs.clone());
        //     }
        //     Ok(())
        // }
        // UpdateEdge but falls back to createEdge if not found
        DotCommand::UpdateEdge { from, to, attrs } => {
            if let Some(edge) = chunks.iter_mut().find(|c| {
                c.kind == "edge" && c.id.as_ref() == Some(from) && c.extra.as_ref() == Some(to)
            }) {
                if let Some(new_attrs) = attrs {
                    edge.attrs = Some(new_attrs.clone());
                }
                Ok(())
            } else {
                // Edge not found, create it
                let line = if chunks.is_empty() {
                    1
                } else {
                    chunks.last().unwrap().range.1 + 1
                };

                chunks.push(Chunk {
                    kind: "edge".to_string(),
                    id: Some(from.clone()),
                    attrs: attrs.clone(),
                    range: (line, line),
                    extra: Some(to.clone()),
                });
                Ok(())
            }
        }
        DotCommand::DeleteEdge { from, to } => {
            let pos = chunks
                .iter()
                .position(|c| {
                    c.kind == "edge" && c.id.as_ref() == Some(from) && c.extra.as_ref() == Some(to)
                })
                .ok_or_else(|| format!("Edge '{}' -> '{}' not found", from, to))?;

            chunks.remove(pos);
            Ok(())
        }

        DotCommand::CreateSubgraph { id, parent } => {
            // Check if subgraph already exists
            if let Some(id) = id
                && chunks
                    .iter()
                    .any(|c| c.kind == "subgraph" && c.id.as_ref() == Some(id))
            {
                return Err(format!("Subgraph '{}' already exists", id));
            }

            // Find insertion point and calculate line range based on parent
            let (insert_pos, line_start, line_end) = if let Some(parent_name) = parent {
                // Find parent subgraph and insert inside it
                let parent_pos = chunks
                    .iter()
                    .position(|c| c.kind == "subgraph" && c.id.as_ref() == Some(parent_name))
                    .ok_or_else(|| format!("Parent subgraph '{}' not found", parent_name))?;

                let parent_range = chunks[parent_pos].range;
                // Insert after parent subgraph declaration, give it a range inside parent
                (parent_pos + 1, parent_range.0 + 1, parent_range.1 - 1)
            } else {
                // Insert at end for top-level subgraph
                let line = if chunks.is_empty() {
                    1
                } else {
                    chunks.last().unwrap().range.1 + 1
                };
                (chunks.len(), line, line + 10) // Give it a 10-line range by default
            };

            chunks.insert(
                insert_pos,
                Chunk {
                    kind: "subgraph".to_string(),
                    id: id.clone(),
                    attrs: None,
                    range: (line_start, line_end),
                    extra: None,
                },
            );

            Ok(())
        }

        DotCommand::DeleteSubgraph { id } => {
            // Find the subgraph
            let subgraph_pos = chunks
                .iter()
                .position(|c| c.kind == "subgraph" && c.id.as_ref() == Some(id))
                .ok_or_else(|| format!("Subgraph '{}' not found", id))?;

            let subgraph_range = chunks[subgraph_pos].range;

            // Remove subgraph and all its children (chunks within its line range)
            chunks.retain(|c| !(c.range.0 >= subgraph_range.0 && c.range.1 <= subgraph_range.1));

            Ok(())
        }

        DotCommand::SetGraphAttr { key, value } => {
            // Look for existing id_eq with this key
            if let Some(attr) = chunks
                .iter_mut()
                .find(|c| c.kind == "id_eq" && c.id.as_ref() == Some(key))
            {
                attr.attrs = Some(value.clone());
            } else {
                // Create new id_eq at beginning
                chunks.insert(
                    0,
                    Chunk {
                        kind: "id_eq".to_string(),
                        id: Some(key.clone()),
                        attrs: Some(value.clone()),
                        range: (1, 1),
                        extra: None,
                    },
                );
            }
            Ok(())
        }

        DotCommand::SetNodeDefault { attrs } => {
            // Look for existing node attr_stmt
            if let Some(attr) = chunks
                .iter_mut()
                .find(|c| c.kind == "attr_stmt" && c.id.as_ref() == Some(&"node".to_string()))
            {
                attr.attrs = Some(attrs.clone());
            } else {
                // Create new node attr_stmt
                let insert_pos = chunks
                    .iter()
                    .position(|c| c.kind == "attr_stmt")
                    .map(|pos| pos + 1)
                    .unwrap_or(0);

                chunks.insert(
                    insert_pos,
                    Chunk {
                        kind: "attr_stmt".to_string(),
                        id: Some("node".to_string()),
                        attrs: Some(attrs.clone()),
                        range: (1, 1),
                        extra: None,
                    },
                );
            }
            Ok(())
        }

        DotCommand::SetEdgeDefault { attrs } => {
            // Look for existing edge attr_stmt
            if let Some(attr) = chunks
                .iter_mut()
                .find(|c| c.kind == "attr_stmt" && c.id.as_ref() == Some(&"edge".to_string()))
            {
                attr.attrs = Some(attrs.clone());
            } else {
                // Create new edge attr_stmt
                let insert_pos = chunks
                    .iter()
                    .position(|c| c.kind == "attr_stmt")
                    .map(|pos| pos + 1)
                    .unwrap_or(0);

                chunks.insert(
                    insert_pos,
                    Chunk {
                        kind: "attr_stmt".to_string(),
                        id: Some("edge".to_string()),
                        attrs: Some(attrs.clone()),
                        range: (1, 1),
                        extra: None,
                    },
                );
            }
            Ok(())
        }

        DotCommand::DeleteAttr { key } => {
            let pos = chunks
                .iter()
                .position(|c| c.kind == "id_eq" && c.id.as_ref() == Some(key))
                .ok_or_else(|| format!("Attribute '{}' not found", key))?;

            chunks.remove(pos);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunks() -> Vec<Chunk> {
        vec![
            Chunk {
                kind: "node".to_string(),
                id: Some("A".to_string()),
                attrs: Some(r#"label="Node A""#.to_string()),
                range: (1, 1),
                extra: None,
            },
            Chunk {
                kind: "node".to_string(),
                id: Some("B".to_string()),
                attrs: Some(r#"label="Node B""#.to_string()),
                range: (2, 2),
                extra: None,
            },
            Chunk {
                kind: "edge".to_string(),
                id: Some("A".to_string()),
                attrs: Some(r#"label="A to B""#.to_string()),
                range: (3, 3),
                extra: Some("B".to_string()),
            },
        ]
    }

    #[test]
    fn test_create_node() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::CreateNode {
            id: "C".to_string(),
            attrs: Some(r#"label="Node C" shape=box"#.to_string()),
            parent: None,
        };

        apply_command(&mut chunks, &cmd).unwrap();
        assert_eq!(chunks.len(), 4);
        assert!(
            chunks
                .iter()
                .any(|c| c.id.as_ref() == Some(&"C".to_string()))
        );
    }

    #[test]
    fn test_update_node() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::UpdateNode {
            id: "A".to_string(),
            attrs: Some(r#"label="Updated A" color=red"#.to_string()),
        };

        apply_command(&mut chunks, &cmd).unwrap();
        let node = chunks
            .iter()
            .find(|c| c.id.as_ref() == Some(&"A".to_string()))
            .unwrap();
        assert!(node.attrs.as_ref().unwrap().contains("Updated A"));
    }

    #[test]
    fn test_delete_node() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::DeleteNode {
            id: "A".to_string(),
        };

        apply_command(&mut chunks, &cmd).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(
            !chunks
                .iter()
                .any(|c| c.id.as_ref() == Some(&"A".to_string()))
        );
    }

    #[test]
    fn test_create_edge() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::CreateEdge {
            from: "B".to_string(),
            to: "A".to_string(),
            attrs: Some(r#"label="B to A" style=dashed"#.to_string()),
            parent: None,
        };

        apply_command(&mut chunks, &cmd).unwrap();
        assert_eq!(chunks.len(), 4);
        assert!(chunks.iter().any(|c| {
            c.kind == "edge"
                && c.id.as_ref() == Some(&"B".to_string())
                && c.extra.as_ref() == Some(&"A".to_string())
        }));
    }

    #[test]
    fn test_update_edge() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::UpdateEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            attrs: Some(r#"label="Updated edge" color=blue"#.to_string()),
        };

        apply_command(&mut chunks, &cmd).unwrap();
        let edge = chunks
            .iter()
            .find(|c| c.kind == "edge" && c.id.as_ref() == Some(&"A".to_string()))
            .unwrap();
        assert!(edge.attrs.as_ref().unwrap().contains("Updated edge"));
    }

    #[test]
    fn test_delete_edge() {
        let mut chunks = create_test_chunks();
        let cmd = DotCommand::DeleteEdge {
            from: "A".to_string(),
            to: "B".to_string(),
        };

        apply_command(&mut chunks, &cmd).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(!chunks.iter().any(|c| c.kind == "edge"));
    }

    #[test]
    fn test_json_serialization() {
        let cmd = DotCommand::CreateNode {
            id: "TestNode".to_string(),
            attrs: Some(r#"label="Test""#.to_string()),
            parent: None,
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("create_node"));
        assert!(json.contains("TestNode"));

        let deserialized: DotCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            DotCommand::CreateNode { id, .. } => assert_eq!(id, "TestNode"),
            _ => panic!("Wrong command type"),
        }
    }
}
