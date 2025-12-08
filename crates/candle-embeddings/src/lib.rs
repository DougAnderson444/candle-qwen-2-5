mod error;
pub use error::Error;

mod model;
pub use model::Model;

#[cfg(feature = "tokio")]
pub mod model_args;
#[cfg(feature = "tokio")]
pub use model_args::ModelArgs;

mod embeddings;
pub use embeddings::Embeddings;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_loading() {
        // Example test to load a model
    }
}
