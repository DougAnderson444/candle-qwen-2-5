//! Library which uses candle to load and run Qwen2.5 models in GGUF format.
use anyhow::Result;
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

use candle::{quantized::gguf_file, Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};

use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;

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

pub fn device(cpu: bool) -> candle::Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else {
        let device = Device::cuda_if_available(0)?;
        Ok(device)
    }
}

pub struct GenerationStats {
    pub prompt_tokens: usize,
    pub prompt_processing_time: std::time::Duration,
    pub generated_tokens: usize,
    pub generation_time: std::time::Duration,
}

pub struct Qwen2Model {
    model: Qwen2,
    device: Device,
    tokenizer: Tokenizer,
    logits_processor: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
    eos_token: u32,
    split_prompt: bool,
}

impl Qwen2Model {
    pub fn new(args: &ModelArgs) -> Result<Self> {
        let device = device(args.cpu)?;
        let model_path = args.model()?;
        let mut file = std::fs::File::open(&model_path)?;
        let model = {
            let model = gguf_file::Content::read(&mut file).map_err(|e| e.with_path(model_path))?;
            Qwen2::from_gguf(model, &mut file, &device)?
        };

        let tokenizer = args.tokenizer()?;
        let logits_processor = {
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

        let eos_token = *tokenizer.get_vocab(true).get("<|im_end|>").unwrap();

        Ok(Self {
            model,
            device,
            tokenizer,
            logits_processor,
            repeat_penalty: args.repeat_penalty,
            repeat_last_n: args.repeat_last_n,
            eos_token,
            split_prompt: args.split_prompt,
        })
    }

    pub fn generate<F: FnMut(String) -> Result<()>>(
        &mut self,
        prompt: &str,
        sample_len: usize,
        mut callback: F,
    ) -> Result<GenerationStats> {
        let mut tos = TokenOutputStream::new(self.tokenizer.clone());
        let prompt_str = format!("<|im_start|>user\n{prompt}<|im_end|>\n<|im_start|>assistant\n");

        let tokens = self
            .tokenizer
            .encode(prompt_str.as_str(), true)
            .map_err(anyhow::Error::msg)?;

        let tokens = tokens.get_ids();

        let to_sample = sample_len.saturating_sub(1);

        let mut all_tokens = vec![];

        let start_prompt_processing = std::time::Instant::now();

        let mut next_token = if !self.split_prompt {
            let input = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, 0)?;
            let logits = logits.squeeze(0)?;
            self.logits_processor.sample(&logits)?
        } else {
            let mut next_token = 0;
            for (pos, token) in tokens.iter().enumerate() {
                let input = Tensor::new(&[*token], &self.device)?.unsqueeze(0)?;
                let logits = self.model.forward(&input, pos)?;
                let logits = logits.squeeze(0)?;
                next_token = self.logits_processor.sample(&logits)?;
            }
            next_token
        };

        let prompt_dt = start_prompt_processing.elapsed();

        all_tokens.push(next_token);

        if let Some(t) = tos.next_token(next_token)? {
            callback(t)?;
        }

        let eos_token = self.eos_token;

        let start_post_prompt = std::time::Instant::now();

        let mut sampled = 0;
        for _index in 0..to_sample {
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, tokens.len() + sampled)?;
            let logits = logits.squeeze(0)?;
            let logits = if self.repeat_penalty == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(self.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.repeat_penalty,
                    &all_tokens[start_at..],
                )?
            };
            next_token = self.logits_processor.sample(&logits)?;
            all_tokens.push(next_token);
            if let Some(t) = tos.next_token(next_token)? {
                callback(t)?;
            }
            sampled += 1;
            if next_token == eos_token {
                break;
            };
        }

        if let Some(rest) = tos.decode_rest().map_err(candle::Error::msg)? {
            callback(rest)?;
        }

        let dt = start_post_prompt.elapsed();
        Ok(GenerationStats {
            prompt_tokens: tokens.len(),
            prompt_processing_time: prompt_dt,
            generated_tokens: sampled,
            generation_time: dt,
        })
    }
}
