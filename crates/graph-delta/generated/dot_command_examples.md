# DOT Command Examples

These are valid JSON commands for modifying DOT graphs.
Generated automatically - do not edit manually.

## Create a node

```json
{
  "action": "create_node",
  "attrs": "label=\"My Node\" shape=box fillcolor=\"#ccffcc\"",
  "id": "NodeA"
}
```

## Create a node inside a subgraph

```json
{
  "action": "create_node",
  "attrs": "label=\"Inside Cluster\"",
  "id": "NodeB",
  "parent": "cluster_Main"
}
```

## Update a node's attributes

```json
{
  "action": "update_node",
  "attrs": "label=\"Updated\" color=red",
  "id": "NodeA"
}
```

## Delete a node

```json
{
  "action": "delete_node",
  "id": "NodeA"
}
```

## Create an edge

```json
{
  "action": "create_edge",
  "attrs": "label=\"connects\" color=blue",
  "from": "NodeA",
  "to": "NodeB"
}
```

## Create an edge with port

```json
{
  "action": "create_edge",
  "attrs": "label=\"port connection\"",
  "from": "NodeA:p1",
  "to": "NodeB:p2"
}
```

## Create an edge inside a subgraph

```json
{
  "action": "create_edge",
  "attrs": "label=\"internal\"",
  "from": "NodeA",
  "parent": "cluster_Main",
  "to": "NodeB"
}
```

## Update an edge

```json
{
  "action": "update_edge",
  "attrs": "label=\"modified\" style=dashed",
  "from": "NodeA",
  "to": "NodeB"
}
```

## Delete an edge

```json
{
  "action": "delete_edge",
  "from": "NodeA",
  "to": "NodeB"
}
```

## Create a subgraph/cluster

```json
{
  "action": "create_subgraph",
  "id": "cluster_Main"
}
```

## Create a nested subgraph

```json
{
  "action": "create_subgraph",
  "id": "cluster_Inner",
  "parent": "cluster_Main"
}
```

## Create anonymous subgraph (rank constraint)

```json
{
  "action": "create_subgraph"
}
```

## Delete a subgraph

```json
{
  "action": "delete_subgraph",
  "id": "cluster_Main"
}
```

## Set graph-level attribute

```json
{
  "action": "set_graph_attr",
  "key": "rankdir",
  "value": "LR"
}
```

## Set default node attributes

```json
{
  "action": "set_node_default",
  "attrs": "shape=box style=filled fillcolor=\"#e8f4ff\""
}
```

## Set default edge attributes

```json
{
  "action": "set_edge_default",
  "attrs": "color=\"#666666\" arrowsize=0.9"
}
```

## Delete a graph attribute

```json
{
  "action": "delete_attr",
  "key": "rankdir"
}
```

