//! A BERT model for generating sentence embeddings.
use crate::{Embeddings, Error};
use candle::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::{PaddingParams, Tokenizer};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Params {
    sentences: Vec<String>,
    normalize_embeddings: bool,
}

pub struct Model {
    bert: BertModel,
    tokenizer: Tokenizer,
}

impl Model {
    /// Load a BERT model from the given weights, tokenizer, and config.
    pub fn new_from_bytes(
        weights: Vec<u8>,
        tokenizer: Vec<u8>,
        config: Vec<u8>,
    ) -> Result<Model, Error> {
        let device = &Device::Cpu;
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, device)?;
        let config: Config = serde_json::from_slice(&config)?;
        let tokenizer =
            Tokenizer::from_bytes(&tokenizer).map_err(|m| Error::Tokenizer(m.to_string()))?;
        let bert = BertModel::load(vb, &config)?;

        Ok(Self { bert, tokenizer })
    }

    /// New from Types
    pub fn new(bert: BertModel, tokenizer: Tokenizer) -> Self {
        Self { bert, tokenizer }
    }

    pub fn get_embeddings(&mut self, input: Params) -> Result<Embeddings, Error> {
        let sentences = input.sentences;
        let normalize_embeddings = input.normalize_embeddings;

        let device = &Device::Cpu;
        if let Some(pp) = self.tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            self.tokenizer.with_padding(Some(pp));
        }
        let tokens = self
            .tokenizer
            .encode_batch(sentences.to_vec(), true)
            .map_err(|m| Error::EncodeBatch(m.to_string()))?;

        let token_ids: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let attention_mask: Vec<Tensor> = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_attention_mask().to_vec();
                Tensor::new(tokens.as_slice(), device)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let attention_mask = Tensor::stack(&attention_mask, 0)?;
        let token_type_ids = token_ids.zeros_like()?;
        let embeddings = self
            .bert
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))?;
        // Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = if normalize_embeddings {
            embeddings.broadcast_div(&embeddings.sqr()?.sum_keepdim(1)?.sqrt()?)?
        } else {
            embeddings
        };
        let embeddings_data = embeddings.to_vec2()?;
        Ok(Embeddings {
            data: embeddings_data,
        })
    }
}
