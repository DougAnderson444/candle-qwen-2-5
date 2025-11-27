run:
  just candle-qwen2-5-cli/run

# runs the Dioxus app
app:
  just crates/api-server/build-release
  just crates/app/serve

single:
  cargo test parser::tests::test_parse_dot_to_chunks_kitchen_sink --lib -- --nocapture
