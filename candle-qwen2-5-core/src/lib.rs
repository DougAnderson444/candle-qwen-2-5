//! Library which uses candle to load and run Qwen2.5 models in GGUF format.
use anyhow::Result;
use hf_hub::api::sync::Api;
use std::io::Write;
use tokenizers::Tokenizer;

use candle::{quantized::gguf_file, Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};

use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;

pub const DEFAULT_PROMPT: &str =
    "Write a Rust function to calculate the factorial of a given number.";

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Which {
    W25_0_5b,
    W25_1_5b,
    W25_3b,
    W25_7b,
}

#[derive(Debug)]
pub struct ModelArgs {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub sample_len: usize,
    pub tokenizer: Option<String>,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub seed: u64,
    pub tracing: bool,
    pub split_prompt: bool,
    pub cpu: bool,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    pub which: Which,
}

impl ModelArgs {
    fn tokenizer(&self) -> Result<Tokenizer> {
        let tokenizer_path = match &self.tokenizer {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = Api::new()?;
                let repo = match self.which {
                    Which::W25_0_5b => "Qwen/Qwen2.5-0.5B-Instruct",
                    Which::W25_1_5b => "Qwen/Qwen2.5-1.5B-Instruct",
                    Which::W25_3b => "Qwen/Qwen2.5-3B-Instruct",
                    Which::W25_7b => "Qwen/Qwen2.5-7B-Instruct",
                };
                let api = api.model(repo.to_string());
                api.get("tokenizer.json")?
            }
        };
        Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)
    }

    fn model(&self) -> Result<std::path::PathBuf> {
        let model_path = match &self.model {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let (repo, filename) = match self.which {
                    Which::W25_0_5b => (
                        "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                        "qwen2.5-0.5b-instruct-q4_k_m.gguf",
                    ),
                    Which::W25_1_5b => (
                        "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
                        "qwen2.5-1.5b-instruct-q4_k_m.gguf",
                    ),
                    Which::W25_3b => (
                        "Qwen/Qwen2.5-3B-Instruct-GGUF",
                        "qwen2.5-3b-instruct-q4_k_m.gguf",
                    ),
                    Which::W25_7b => (
                        "Qwen/Qwen2.5-7B-Instruct-GGUF",
                        "qwen2.5-7b-instruct-q4_k_m.gguf",
                    ),
                };
                let api = Api::new()?;
                api.model(repo.to_string()).get(filename)?
            }
        };
        Ok(model_path)
    }
}

/// This is a wrapper around a tokenizer to ensure that tokens can be returned to the user in a
/// streaming way rather than having to wait for the full decoding.
pub struct TokenOutputStream {
    tokenizer: tokenizers::Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl TokenOutputStream {
    pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
        Self {
            tokenizer,
            tokens: Vec::new(),
            prev_index: 0,
            current_index: 0,
        }
    }

    fn decode(&self, tokens: &[u32]) -> candle::Result<String> {
        match self.tokenizer.decode(tokens, true) {
            Ok(str) => Ok(str),
            Err(err) => candle::bail!("cannot decode: {err}"),
        }
    }

    // https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68
    pub fn next_token(&mut self, token: u32) -> candle::Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
            let text = text.split_at(prev_text.len());
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn decode_rest(&self) -> candle::Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() {
            let text = text.split_at(prev_text.len());
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }
}

fn format_size(size_in_bytes: usize) -> String {
    if size_in_bytes < 1_000 {
        format!("{size_in_bytes}B")
    } else if size_in_bytes < 1_000_000 {
        format!("{:.2}KB", size_in_bytes as f64 / 1e3)
    } else if size_in_bytes < 1_000_000_000 {
        format!("{:.2}MB", size_in_bytes as f64 / 1e6)
    } else {
        format!("{:.2}GB", size_in_bytes as f64 / 1e9)
    }
}

pub fn device(cpu: bool) -> candle::Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else {
        let device = Device::cuda_if_available(0)?;
        Ok(device)
    }
}

pub fn run(args: ModelArgs) -> Result<()> {
    let model_path = args.model()?;
    let mut file = std::fs::File::open(&model_path)?;
    let device = device(args.cpu)?;

    let mut model = {
        let model = gguf_file::Content::read(&mut file).map_err(|e| e.with_path(model_path))?;
        let mut total_size_in_bytes = 0;
        for (_, tensor) in model.tensor_infos.iter() {
            let elem_count = tensor.shape.elem_count();
            total_size_in_bytes +=
                elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.block_size();
        }
        Qwen2::from_gguf(model, &mut file, &device)?
    };

    let tokenizer = args.tokenizer()?;
    let mut tos = TokenOutputStream::new(tokenizer);
    let prompt_str = args
        .prompt
        .clone()
        .unwrap_or_else(|| DEFAULT_PROMPT.to_string());

    let prompt_str = format!("<|im_start|>user\n{prompt_str}<|im_end|>\n<|im_start|>assistant\n");

    let tokens = tos
        .tokenizer()
        .encode(prompt_str.as_str(), true)
        .map_err(anyhow::Error::msg)?;

    let tokens = tokens.get_ids();

    let to_sample = args.sample_len.saturating_sub(1);

    let mut all_tokens = vec![];

    let mut logits_processor = {
        let temperature = args.temperature;
        let sampling = if temperature <= 0. {
            Sampling::ArgMax
        } else {
            match (args.top_k, args.top_p) {
                (None, None) => Sampling::All { temperature },
                (Some(k), None) => Sampling::TopK { k, temperature },
                (None, Some(p)) => Sampling::TopP { p, temperature },
                (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
            }
        };
        LogitsProcessor::from_sampling(args.seed, sampling)
    };

    let start_prompt_processing = std::time::Instant::now();

    let mut next_token = if !args.split_prompt {
        let input = Tensor::new(tokens, &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, 0)?;
        let logits = logits.squeeze(0)?;
        logits_processor.sample(&logits)?
    } else {
        let mut next_token = 0;
        for (pos, token) in tokens.iter().enumerate() {
            let input = Tensor::new(&[*token], &device)?.unsqueeze(0)?;
            let logits = model.forward(&input, pos)?;
            let logits = logits.squeeze(0)?;
            next_token = logits_processor.sample(&logits)?;
        }
        next_token
    };

    let prompt_dt = start_prompt_processing.elapsed();

    all_tokens.push(next_token);

    if let Some(t) = tos.next_token(next_token)? {
        print!("{t}");
        std::io::stdout().flush()?;
    }

    let eos_token = *tos.tokenizer().get_vocab(true).get("<|im_end|>").unwrap();

    let start_post_prompt = std::time::Instant::now();

    let mut sampled = 0;
    for _index in 0..to_sample {
        let input = Tensor::new(&[next_token], &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, tokens.len() + sampled)?;
        let logits = logits.squeeze(0)?;
        let logits = if args.repeat_penalty == 1. {
            logits
        } else {
            let start_at = all_tokens.len().saturating_sub(args.repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                args.repeat_penalty,
                &all_tokens[start_at..],
            )?
        };
        next_token = logits_processor.sample(&logits)?;
        all_tokens.push(next_token);
        if let Some(t) = tos.next_token(next_token)? {
            print!("{t}");
            std::io::stdout().flush()?;
        }
        sampled += 1;
        if next_token == eos_token {
            break;
        };
    }

    if let Some(rest) = tos.decode_rest().map_err(candle::Error::msg)? {
        print!("{rest}");
    }

    std::io::stdout().flush()?;
    let dt = start_post_prompt.elapsed();
    println!(
        "\n\n{:4} prompt tokens processed: {:.2} token/s",
        tokens.len(),
        tokens.len() as f64 / prompt_dt.as_secs_f64(),
    );
    println!(
        "{:4} tokens generated: {:.2} token/s",
        sampled,
        sampled as f64 / dt.as_secs_f64(),
    );
    Ok(())
}
