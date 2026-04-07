use serde::{Deserialize, Serialize};

use crate::embedder::{cosine_similarity, Embedder};
use crate::error::CompilerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum SafetyAction {
    /// Emit a warning but return the compiled output
    Warn,
    /// Fall back to the original uncompiled prompt
    Fallback,
    /// Abort with an error
    Abort,
}

impl Default for SafetyAction {
    fn default() -> Self {
        SafetyAction::Warn
    }
}

#[derive(Debug, Clone)]
pub struct SafetyCheck {
    pub min_semantic_similarity: f64,
    pub on_fail: SafetyAction,
}

impl Default for SafetyCheck {
    fn default() -> Self {
        Self {
            min_semantic_similarity: 0.85,
            on_fail: SafetyAction::Warn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyResult {
    pub similarity: f64,
    pub passed: bool,
    pub warning: Option<String>,
}

impl SafetyCheck {
    pub fn new(threshold: f64, on_fail: SafetyAction) -> Self {
        Self {
            min_semantic_similarity: threshold,
            on_fail,
        }
    }

    /// Check semantic similarity between original and compiled text.
    /// Returns the safety result and optionally the fallback text.
    pub fn check(
        &self,
        original: &str,
        compiled: &str,
        embedder: &dyn Embedder,
    ) -> Result<SafetyResult, CompilerError> {
        let original_emb = embedder.embed(original);
        let compiled_emb = embedder.embed(compiled);
        let similarity = cosine_similarity(&original_emb, &compiled_emb) as f64;

        if similarity >= self.min_semantic_similarity {
            return Ok(SafetyResult {
                similarity,
                passed: true,
                warning: None,
            });
        }

        match self.on_fail {
            SafetyAction::Warn => Ok(SafetyResult {
                similarity,
                passed: false,
                warning: Some(format!(
                    "Semantic drift detected: similarity {:.3} < threshold {:.3}. \
                     The compiled prompt may have diverged from the original intent.",
                    similarity, self.min_semantic_similarity
                )),
            }),
            SafetyAction::Fallback => Ok(SafetyResult {
                similarity,
                passed: false,
                warning: Some(format!(
                    "Semantic drift detected (similarity {:.3}). Falling back to original prompt.",
                    similarity
                )),
            }),
            SafetyAction::Abort => Err(CompilerError::SemanticDrift {
                similarity,
                threshold: self.min_semantic_similarity,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::tfidf::TfIdfEmbedder;

    #[test]
    fn test_safety_pass_similar_texts() {
        let docs = &[
            "Write clear and concise summaries",
            "Write clear, concise summaries for users",
        ];
        let embedder = TfIdfEmbedder::from_documents(docs);
        let check = SafetyCheck::new(0.5, SafetyAction::Warn);

        let result = check.check(docs[0], docs[1], &embedder).unwrap();
        assert!(result.passed, "similarity={}", result.similarity);
    }

    #[test]
    fn test_safety_fail_different_texts() {
        let docs = &[
            "Write about machine learning algorithms in detail",
            "Cook a delicious pasta recipe for dinner tonight",
        ];
        let embedder = TfIdfEmbedder::from_documents(docs);
        let check = SafetyCheck::new(0.9, SafetyAction::Warn);

        let result = check.check(docs[0], docs[1], &embedder).unwrap();
        assert!(!result.passed);
        assert!(result.warning.is_some());
    }

    #[test]
    fn test_safety_abort_returns_error() {
        let docs = &[
            "Write about machine learning algorithms in detail",
            "Cook a delicious pasta recipe for dinner tonight",
        ];
        let embedder = TfIdfEmbedder::from_documents(docs);
        let check = SafetyCheck::new(0.9, SafetyAction::Abort);

        let result = check.check(docs[0], docs[1], &embedder);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilerError::SemanticDrift { .. } => {}
            other => panic!("Expected SemanticDrift, got: {other}"),
        }
    }

    #[test]
    fn test_safety_fallback_returns_warning() {
        let docs = &[
            "Write about machine learning algorithms in detail",
            "Cook a delicious pasta recipe for dinner tonight",
        ];
        let embedder = TfIdfEmbedder::from_documents(docs);
        let check = SafetyCheck::new(0.9, SafetyAction::Fallback);

        let result = check.check(docs[0], docs[1], &embedder).unwrap();
        assert!(!result.passed);
        assert!(result.warning.as_ref().unwrap().contains("Falling back"));
    }
}
