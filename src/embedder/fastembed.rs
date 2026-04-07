#[cfg(feature = "fastembed")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

#[cfg(feature = "fastembed")]
use super::Embedder;

/// Neural embedder using fastembed-rs (all-MiniLM-L6-v2, 22MB ONNX model).
/// Enable with `--features fastembed`.
#[cfg(feature = "fastembed")]
pub struct FastEmbedder {
    model: TextEmbedding,
}

#[cfg(feature = "fastembed")]
impl FastEmbedder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )?;
        Ok(Self { model })
    }

    pub fn with_progress() -> Result<Self, Box<dyn std::error::Error>> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        let texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        self.model
            .embed(texts, None)
            .unwrap_or_default()
    }
}

#[cfg(feature = "fastembed")]
impl Embedder for FastEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.model
            .embed(vec![text.to_string()], None)
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}
