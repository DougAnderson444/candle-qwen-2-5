//! Tool definitions and handling for LLM graph modification
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::DotCommand;
use crate::parser::Chunk;

/// Tool definitions that the LLM can call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Get tool definitions for the LLM
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_node".to_string(),
            description: "Get details about a specific node in the graph".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The node ID to query"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "list_nodes".to_string(),
            description: "List all nodes in the graph or within a specific subgraph".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "parent": {
                        "type": "string",
                        "description": "Optional: parent subgraph to filter by"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "get_edges".to_string(),
            description: "Get all edges connected to a node".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "Node ID to get edges for"
                    }
                },
                "required": ["node_id"]
            }),
        },
        ToolDefinition {
            name: "create_node".to_string(),
            description: "Create a new node in the graph".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Unique identifier for the node"
                    },
                    "label": {
                        "type": "string",
                        "description": "Display label for the node"
                    },
                    "shape": {
                        "type": "string",
                        "description": "Node shape (box, circle, ellipse, etc)",
                        "enum": ["box", "circle", "ellipse", "diamond", "cylinder"]
                    },
                    "color": {
                        "type": "string",
                        "description": "Node color (hex or name)"
                    },
                    "parent": {
                        "type": "string",
                        "description": "Parent subgraph to place node in"
                    }
                },
                "required": ["id", "label"]
            }),
        },
        ToolDefinition {
            name: "update_node".to_string(),
            description: "Update an existing node's properties".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Node ID to update"
                    },
                    "label": {
                        "type": "string",
                        "description": "New display label"
                    },
                    "shape": {
                        "type": "string",
                        "description": "New node shape"
                    },
                    "color": {
                        "type": "string",
                        "description": "New node color"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "delete_node".to_string(),
            description: "Remove a node from the graph".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Node ID to delete"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "create_edge".to_string(),
            description: "Create an edge between two nodes".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "from": {
                        "type": "string",
                        "description": "Source node ID"
                    },
                    "to": {
                        "type": "string",
                        "description": "Target node ID"
                    },
                    "label": {
                        "type": "string",
                        "description": "Edge label"
                    },
                    "color": {
                        "type": "string",
                        "description": "Edge color"
                    }
                },
                "required": ["from", "to"]
            }),
        },
        ToolDefinition {
            name: "delete_edge".to_string(),
            description: "Remove an edge between two nodes".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "from": {
                        "type": "string",
                        "description": "Source node ID"
                    },
                    "to": {
                        "type": "string",
                        "description": "Target node ID"
                    }
                },
                "required": ["from", "to"]
            }),
        },
        ToolDefinition {
            name: "create_cluster".to_string(),
            description: "Create a new cluster/subgraph to group nodes".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Cluster identifier (use cluster_ prefix)"
                    },
                    "label": {
                        "type": "string",
                        "description": "Cluster display label"
                    }
                },
                "required": ["id", "label"]
            }),
        },
    ]
}

/// Convert tool call parameters to DotCommand
pub fn tool_call_to_command(
    tool_name: &str,
    params: serde_json::Value,
) -> Result<DotCommand, String> {
    match tool_name {
        "create_node" => {
            let id = params["id"]
                .as_str()
                .ok_or("Missing 'id' parameter")?
                .to_string();

            let mut attrs = Vec::new();

            if let Some(label) = params["label"].as_str() {
                attrs.push(format!("label=\"{}\"", label));
            }
            if let Some(shape) = params["shape"].as_str() {
                attrs.push(format!("shape={}", shape));
            }
            if let Some(color) = params["color"].as_str() {
                attrs.push(format!("color=\"{}\"", color));
            }

            let attrs_str = if attrs.is_empty() {
                None
            } else {
                Some(attrs.join(" "))
            };

            let parent = params["parent"].as_str().map(|s| s.to_string());

            Ok(DotCommand::CreateNode {
                id,
                attrs: attrs_str,
                parent,
            })
        }

        "update_node" => {
            let id = params["id"]
                .as_str()
                .ok_or("Missing 'id' parameter")?
                .to_string();

            let mut attrs = Vec::new();

            if let Some(label) = params["label"].as_str() {
                attrs.push(format!("label=\"{}\"", label));
            }
            if let Some(shape) = params["shape"].as_str() {
                attrs.push(format!("shape={}", shape));
            }
            if let Some(color) = params["color"].as_str() {
                attrs.push(format!("color=\"{}\"", color));
            }

            if attrs.is_empty() {
                return Err("No attributes to update".to_string());
            }

            Ok(DotCommand::UpdateNode {
                id,
                attrs: Some(attrs.join(" ")),
            })
        }

        "delete_node" => {
            let id = params["id"]
                .as_str()
                .ok_or("Missing 'id' parameter")?
                .to_string();

            Ok(DotCommand::DeleteNode { id })
        }

        "create_edge" => {
            let from = params["from"]
                .as_str()
                .ok_or("Missing 'from' parameter")?
                .to_string();
            let to = params["to"]
                .as_str()
                .ok_or("Missing 'to' parameter")?
                .to_string();

            let mut attrs = Vec::new();

            if let Some(label) = params["label"].as_str() {
                attrs.push(format!("label=\"{}\"", label));
            }
            if let Some(color) = params["color"].as_str() {
                attrs.push(format!("color=\"{}\"", color));
            }

            let attrs_str = if attrs.is_empty() {
                None
            } else {
                Some(attrs.join(" "))
            };

            let parent = params["parent"].as_str().map(|s| s.to_string());

            Ok(DotCommand::CreateEdge {
                from,
                to,
                attrs: attrs_str,
                parent,
            })
        }

        "delete_edge" => {
            let from = params["from"]
                .as_str()
                .ok_or("Missing 'from' parameter")?
                .to_string();
            let to = params["to"]
                .as_str()
                .ok_or("Missing 'to' parameter")?
                .to_string();

            Ok(DotCommand::DeleteEdge { from, to })
        }

        "create_cluster" => {
            let id = params["id"]
                .as_str()
                .ok_or("Missing 'id' parameter")?
                .to_string();

            Ok(DotCommand::CreateSubgraph {
                id: Some(id),
                parent: None,
            })
        }

        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

/// Query tools - these don't modify the graph, just return info
pub fn execute_query_tool(
    tool_name: &str,
    params: serde_json::Value,
    chunks: &[Chunk],
) -> Result<serde_json::Value, String> {
    match tool_name {
        "get_node" => {
            let id = params["id"].as_str().ok_or("Missing 'id' parameter")?;

            let node = chunks
                .iter()
                .find(|c| c.kind == "node" && c.id.as_ref() == Some(&id.to_string()))
                .ok_or_else(|| format!("Node '{}' not found", id))?;

            Ok(json!({
                "id": node.id,
                "attrs": node.attrs,
                "type": "node"
            }))
        }

        "list_nodes" => {
            let parent = params.get("parent").and_then(|v| v.as_str());

            let nodes: Vec<_> = chunks
                .iter()
                .filter(|c| c.kind == "node")
                .filter(|c| {
                    if let Some(parent_name) = parent {
                        // Check if node is within parent's range
                        if let Some(parent_chunk) = chunks.iter().find(|p| {
                            p.kind == "subgraph" && p.id.as_ref() == Some(&parent_name.to_string())
                        }) {
                            c.range.0 > parent_chunk.range.0 && c.range.1 < parent_chunk.range.1
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                })
                .map(|c| {
                    json!({
                        "id": c.id,
                        "attrs": c.attrs
                    })
                })
                .collect();

            Ok(json!({ "nodes": nodes }))
        }

        "get_edges" => {
            let node_id = params["node_id"]
                .as_str()
                .ok_or("Missing 'node_id' parameter")?;

            let edges: Vec<_> = chunks
                .iter()
                .filter(|c| {
                    c.kind == "edge"
                        && (c.id.as_ref() == Some(&node_id.to_string())
                            || c.extra.as_ref() == Some(&node_id.to_string()))
                })
                .map(|c| {
                    json!({
                        "from": c.id,
                        "to": c.extra,
                        "attrs": c.attrs
                    })
                })
                .collect();

            Ok(json!({ "edges": edges }))
        }

        _ => Err(format!("Unknown query tool: {}", tool_name)),
    }
}

/// System prompt for the LLM
pub fn get_system_prompt() -> String {
    r#"You are a graph modification assistant. Users will ask you to modify DOT graphs.

You have access to tools to query and modify the graph. Use these tools to:
1. Query current graph state (get_node, list_nodes, get_edges)
2. Create new elements (create_node, create_edge, create_cluster)
3. Update existing elements (update_node)
4. Delete elements (delete_node, delete_edge)

When the user asks to modify a graph:
1. First query relevant information if needed
2. Then make the requested modifications
3. Be concise - don't query information you don't need

Example workflow for "add a node called Server connected to DB":
1. Call create_node with id="Server", label="Server"
2. Call create_edge with from="Server", to="DB"

Example workflow for "change node A to be red":
1. Call update_node with id="A", color="red"

Keep responses brief. Focus on the tools, not explanations."#
        .to_string()
}
