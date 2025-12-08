use crate::Error;
use candle::{Device, Tensor};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Embeddings {
    pub(crate) data: Vec<Vec<f32>>,
}

impl Embeddings {
    /// Gets the nth embedding.
    pub fn get(&self, n: usize) -> Result<Tensor, Error> {
        if n >= self.data.len() {
            return Err(Error::IndexOutOfBounds(n));
        }
        let embedding = &self.data[n];
        Tensor::new(embedding.as_slice(), &Device::Cpu).map_err(|_| Error::TensorCreationFailed)
    }
}
