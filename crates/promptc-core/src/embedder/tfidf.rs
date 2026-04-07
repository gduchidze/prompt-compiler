use std::collections::{HashMap, HashSet};

use super::Embedder;

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was", "one", "our",
    "out", "day", "get", "has", "him", "his", "how", "its", "let", "may", "now", "use", "way",
    "who", "did", "this", "that", "with", "from", "they", "will", "been", "have", "more", "when",
    "your", "said", "each", "into", "than", "then", "time", "very", "also", "just", "about",
    "over", "such", "some", "only", "other", "would", "make", "like", "could", "many", "these",
    "after", "them", "being", "most", "which", "their", "were", "what", "there", "should",
];

fn tokenize_text(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .filter(|s| !STOPWORDS.contains(s))
        .map(String::from)
        .collect()
}

pub struct TfIdfEmbedder {
    vocab: HashMap<String, usize>,
    idf: Vec<f32>,
}

impl TfIdfEmbedder {
    pub fn from_documents(docs: &[&str]) -> Self {
        let n = docs.len();
        let mut vocab: HashMap<String, usize> = HashMap::new();
        let mut df: HashMap<String, usize> = HashMap::new();

        for doc in docs {
            let terms = tokenize_text(doc);
            let unique: HashSet<_> = terms.into_iter().collect();
            for term in unique {
                *df.entry(term.clone()).or_insert(0) += 1;
                if !vocab.contains_key(&term) {
                    let idx = vocab.len();
                    vocab.insert(term, idx);
                }
            }
        }

        let vocab_size = vocab.len();
        let mut idf = vec![0.0f32; vocab_size];
        for (term, &col) in &vocab {
            let doc_freq = df.get(term).copied().unwrap_or(0);
            idf[col] = ((n as f32 + 1.0) / (doc_freq as f32 + 1.0)).ln() + 1.0;
        }

        Self { vocab, idf }
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }
}

impl Embedder for TfIdfEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let terms = tokenize_text(text);
        let mut tf: HashMap<&str, usize> = HashMap::new();
        for term in &terms {
            *tf.entry(term.as_str()).or_insert(0) += 1;
        }

        let mut v = vec![0.0f32; self.vocab.len()];
        if v.is_empty() {
            return v;
        }

        for (term, &count) in &tf {
            if let Some(&col) = self.vocab.get(*term) {
                v[col] = (1.0 + (count as f32).ln()) * self.idf[col];
            }
        }

        // L2 normalize
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }

        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::cosine_similarity;

    #[test]
    fn test_identical_texts_similarity_one() {
        let docs = &["write clear summaries", "use formal language"];
        let emb = TfIdfEmbedder::from_documents(docs);
        let v = emb.embed("write clear summaries");
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.01, "sim={sim}");
    }

    #[test]
    fn test_completely_different_similarity_low() {
        let docs = &[
            "write clear summaries about cats",
            "explain quantum physics thoroughly",
        ];
        let emb = TfIdfEmbedder::from_documents(docs);
        let va = emb.embed("write clear summaries about cats");
        let vb = emb.embed("explain quantum physics thoroughly");
        let sim = cosine_similarity(&va, &vb);
        assert!(sim < 0.3, "sim={sim}");
    }

    #[test]
    fn test_similar_texts_high_similarity() {
        let docs = &[
            "always write clear and concise summaries",
            "write concise and clear summary text",
            "explain quantum physics in detail",
        ];
        let emb = TfIdfEmbedder::from_documents(docs);
        let va = emb.embed("always write clear and concise summaries");
        let vb = emb.embed("write concise and clear summary text");
        let sim = cosine_similarity(&va, &vb);
        assert!(sim > 0.3, "sim={sim}");
    }

    #[test]
    fn test_embed_normalizes_to_unit_length() {
        let docs = &["write clear summaries", "use formal language"];
        let emb = TfIdfEmbedder::from_documents(docs);
        let v = emb.embed("write clear summaries");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={norm}");
    }

    #[test]
    fn test_empty_corpus() {
        let docs: &[&str] = &[];
        let emb = TfIdfEmbedder::from_documents(docs);
        let v = emb.embed("anything");
        assert!(v.is_empty());
    }
}
