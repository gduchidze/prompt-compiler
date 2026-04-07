pub mod fastembed;
pub mod tfidf;

pub use tfidf::TfIdfEmbedder;

#[cfg(feature = "fastembed")]
pub use self::fastembed::FastEmbedder;

pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;

    fn similarity(&self, a: &str, b: &str) -> f32 {
        cosine_similarity(&self.embed(a), &self.embed(b))
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
