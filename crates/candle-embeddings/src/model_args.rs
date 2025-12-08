//! Module for model arguments and utilities.
use hf_hub::api::tokio::Api;
use tokenizers::Tokenizer;

/// Wrapper over IntFloat E5 Small V2 model, tokenizer, and config.
#[derive(Clone, serde::Serialize, serde::Deserialize, Copy)]
struct IntFloatE5SmallV2;

impl IntFloatE5SmallV2 {
    pub const CONFIG: &str = "config.json";
    pub const TOKENIZER: &str = "tokenizer.json";
    pub const WEIGHTS: &str = "model.safetensors";
    pub const MODEL: &str = "intfloat/e5-small-v2";

    pub fn default_config() -> &'static str {
        Self::CONFIG
    }

    pub fn default_tokenizer() -> &'static str {
        Self::TOKENIZER
    }

    pub fn default_weights() -> &'static str {
        Self::WEIGHTS
    }

    pub fn default_model() -> &'static str {
        Self::MODEL
    }
}

/// WHich model to use.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum Which {
    /// Intefloat e5 small v2 model.
    IntFloatE5SmallV2(IntFloatE5SmallV2),
}

pub struct ModelArgs {
    /// The model size to use.
    pub which: Which,

    /// Path to the tokenizer file.
    pub tokenizer: Option<String>,

    /// Config for sampling temperature.
    pub config: Option<String>,
}

impl ModelArgs {
    async fn tokenizer(&self) -> anyhow::Result<Tokenizer> {
        let tokenizer_path = match &self.tokenizer {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = Api::new()?;
                let (repo, tokenizer_file) = match self.which {
                    Which::IntFloatE5SmallV2(_) => {
                        (IntFloatE5SmallV2::MODEL, IntFloatE5SmallV2::TOKENIZER)
                    }
                };
                let api = api.model(repo.to_string());
                api.get(tokenizer_file).await?
            }
        };
        Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)
    }

    async fn model(&self) -> anyhow::Result<std::path::PathBuf> {
        let model_path = match &self.config {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = Api::new()?;
                let (repo, model_file) = match self.which {
                    Which::IntFloatE5SmallV2(_) => {
                        (IntFloatE5SmallV2::MODEL, IntFloatE5SmallV2::WEIGHTS)
                    }
                };
                let api = api.model(repo.to_string());
                api.get(model_file).await?
            }
        };
        Ok(model_path)
    }

    async fn config(&self) -> anyhow::Result<std::path::PathBuf> {
        let config_path = match &self.config {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = Api::new()?;
                let (repo, config_file) = match self.which {
                    Which::IntFloatE5SmallV2(_) => {
                        (IntFloatE5SmallV2::MODEL, IntFloatE5SmallV2::CONFIG)
                    }
                };
                let api = api.model(repo.to_string());
                api.get(config_file).await?
            }
        };
        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hub root (default): ~/.cache/huggingface/hub
    // Token file (default): ~/.cache/huggingface/token
    #[tokio::test]
    async fn test_model_args() -> anyhow::Result<()> {
        let args = ModelArgs {
            which: Which::IntFloatE5SmallV2(IntFloatE5SmallV2),
            tokenizer: None,
            config: None,
        };

        let tokenizer = args.tokenizer().await.unwrap();
        assert!(tokenizer.get_vocab_size(false) > 0);

        let model_path = args.model().await.unwrap();
        assert!(model_path.exists());

        let config_path = args.config().await.unwrap();
        assert!(config_path.exists());

        // check that somewhere in ~/.cache/huggingface/hub/models--intfloat--e5-small-v2 the files exist
        let home_dir = dirs::home_dir().unwrap();
        let hub_dir = home_dir.join(".cache/huggingface/hub/models--intfloat--e5-small-v2");
        assert!(hub_dir.exists());

        Ok(())
    }
}
