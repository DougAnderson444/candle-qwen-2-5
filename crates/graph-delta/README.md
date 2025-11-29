# Graph Delta 

Parses a DOT file into a graph structure, enables users to make delta commands to change aspects of the graph, and then outputs a new DOT file reflecting those changes.

## Architecture Overview

```
User Request
    ↓
LLM (Qwen 2.5-Instruct, typically 1.5B model which is faster on CPU)
    ↓
GraphOps DSL
    ↓
DSL Parser (Pest)
    ↓
DslCommand AST
    ↓
Interpreter
    ↓
Chunks (Modified)
    ↓
DOT Serializer
    ↓
Final Graph
```

