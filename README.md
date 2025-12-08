# Candle Machine Learning library Utilities

Project contains the crates:

- Qwen 2.5 [core model code](./candle-qwen2-5-core/)
- Qwen 2.5 [cli](./candle-qwen2-5-cli/)
- Basic [Dioxus App](./app/)
- [Vector Emeddings](./crates/candle-embeddings/) using Candle & Intfloat E5 small v2 model

## Model cache from Hugging Face

The model files are cached from Hugging Face. You can find the model [here](https://huggingface.co/Qwen/Qwen-2.5-Base). The cache directory is usually located at `~/.cache/huggingface/hub/models--Qwen--Qwen-2.5-Base/snapshots/<commit_hash>/`.
