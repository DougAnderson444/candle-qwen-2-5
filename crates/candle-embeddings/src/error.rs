//! crate level errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid model path: {0}")]
    InvalidModelPath(String),
    /// From<candle_core::Error>
    #[error("Candle error: {0}")]
    CandleError(#[from] candle::Error),
    /// From<serde_json::Error>
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// From<tokenizers::Error>
    #[error("Tokenizer error: {0}")]
    Tokenizer(String),
    /// Error during encoding batch
    #[error("Error during encoding batch: {0}")]
    EncodeBatch(String),
    /// Model is not initialized
    #[error("Model is not initialized")]
    ModelNotInitialized,
    /// Index out of bound
    #[error("Index {0} is out of bounds")]
    IndexOutOfBounds(usize),
    /// Tensor failed to create
    #[error("Tensor creation failed")]
    TensorCreationFailed,
}
