# Qwen2.5 GGUF API Server

This crate provides an OpenAI-compatible API server for running Qwen2.5 GGUF models using `candle`.

## Getting Started

### Prerequisites

- Rust and Cargo are installed.
- `just` is installed (`cargo install just`).

### Running the Server

You can run the server using either `just` or `cargo`.

**Using `just`:**

The `justfile` in this directory provides convenient commands:

- `just run`: Run the server in debug mode.
- `just run-release`: Run the server in release mode (recommended for performance).
- `just build`: Build the server in debug mode.
- `just build-release`: Build the server in release mode.
- `just check`: Check the code for errors.

**Using `cargo`:**

- `cargo run`: Run the server in debug mode.
- `cargo run --release`: Run the server in release mode.

The server will start on `0.0.0.0:3000` by default. You can change the port with the `--port` argument.

### Interacting with the API

You can send requests to the `/v1/chat/completions` endpoint.

**Example using `curl` (streaming):**

```bash
curl -N -X POST http://localhost:42069/v1/chat/completions \
-H "Content-Type: application/json" \
-d '{
  "messages": [
    {
      "role": "user",
      "content": "Write a short story about a robot who dreams of becoming a chef."
    }
  ],
  "stream": true,
  "max_tokens": 200
}'
```

**Example using `curl` (non-streaming):**

```bash
curl -X POST http://localhost:42069/v1/chat/completions \
-H "Content-Type: application/json" \
-d '{
  "messages": [
    {
      "role": "user",
      "content": "What is the capital of France?"
    }
  ],
  "stream": false
}'
```
