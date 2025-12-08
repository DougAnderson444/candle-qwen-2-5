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
    use crate::model::Params;

    // Hub root (default): ~/.cache/huggingface/hub
    // Token file (default): ~/.cache/huggingface/token
    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_model_use() {
        let args = ModelArgs::default();
        let mut model = Model::from_args(&args).await.unwrap();

        let sentences = [
            "The cat sits outside",
            "El gato está sentado afuera",
            "A man is playing guitar",
            "Un hombre está tocando la guitarra",
            "I love pasta",
            "Me encanta la pasta",
            "The new movie is awesome",
            "La nueva película es impresionante",
            "The cat plays in the garden",
            "El gato juega en el jardín",
            "A woman watches TV",
            "[translate:Una mujer mira la televisión]",
            "The new movie is so great",
            "La nueva película es tan buena",
            "Do you like pizza?",
            "¿Te gusta la pizza?",
        ];

        let params = Params {
            sentences: sentences.iter().map(|s| s.to_string()).collect(),
            // so we can cosine similarity search between the embeddings
            normalize_embeddings: true,
        };

        let embeddings = model.get_embeddings(params).unwrap();

        let mut similarities = vec![];
        for i in 0..sentences.len() {
            for j in (i + 1)..sentences.len() {
                let vec1 = &embeddings.data[i];
                let vec2 = &embeddings.data[j];
                let dot_product: f32 = vec1.iter().zip(vec2).map(|(a, b)| a * b).sum();
                let norm1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
                let score = dot_product / (norm1 * norm2);
                similarities.push((score, i, j));
            }
        }

        similarities.sort_by(|u, v| v.0.total_cmp(&u.0));
        for &(score, i, j) in similarities[..5].iter() {
            println!("score: {score:.2} '{}' '{}'", sentences[i], sentences[j])
        }
    }
}
