run:
  just candle-qwen2-5-cli/run

# runs the Dioxus app
app:
  just crates/api-server/build-release
  just crates/app/serve
