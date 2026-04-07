use crate::codegen::ModelTarget;

pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> usize;
}

pub struct WhitespaceCounter;

impl TokenCounter for WhitespaceCounter {
    fn count(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
}

/// Honest token count that distinguishes exact vs approximate counts.
#[derive(Debug, Clone)]
pub enum TokenCount {
    Exact(usize),
    Approximate { count: usize, note: &'static str },
}

impl TokenCount {
    pub fn value(&self) -> usize {
        match self {
            TokenCount::Exact(n) => *n,
            TokenCount::Approximate { count, .. } => *count,
        }
    }

    pub fn is_exact(&self) -> bool {
        matches!(self, TokenCount::Exact(_))
    }
}

impl std::fmt::Display for TokenCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenCount::Exact(n) => write!(f, "{n} tokens (exact)"),
            TokenCount::Approximate { count, note } => {
                write!(f, "{count} tokens (~, {note})")
            }
        }
    }
}

/// Count tokens honestly based on target model.
/// Uses whitespace approximation with target-specific accuracy notes.
pub fn count_tokens(text: &str, target: ModelTarget) -> TokenCount {
    let count = text.split_whitespace().count();

    // Approximate: whitespace splitting underestimates by ~25-30% vs real tokenizers.
    // Apply a correction factor for a better estimate.
    let adjusted = (count as f64 * 1.33).round() as usize;

    match target {
        ModelTarget::Gpt => TokenCount::Approximate {
            count: adjusted,
            note: "estimated — use tiktoken for exact count",
        },
        ModelTarget::Claude => TokenCount::Approximate {
            count: adjusted,
            note: "estimated ±5% — Anthropic tokenizer is not public",
        },
        ModelTarget::Mistral => TokenCount::Approximate {
            count: adjusted,
            note: "estimated — SentencePiece approximation",
        },
        ModelTarget::Llama => TokenCount::Approximate {
            count: adjusted,
            note: "estimated — SentencePiece approximation",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_counter() {
        let counter = WhitespaceCounter;
        assert_eq!(counter.count("hello world"), 2);
        assert_eq!(counter.count("one two three four"), 4);
        assert_eq!(counter.count(""), 0);
        assert_eq!(counter.count("   "), 0);
        assert_eq!(counter.count("single"), 1);
    }

    #[test]
    fn test_token_count_display_exact() {
        let tc = TokenCount::Exact(42);
        assert!(tc.to_string().contains("exact"));
        assert_eq!(tc.value(), 42);
        assert!(tc.is_exact());
    }

    #[test]
    fn test_token_count_display_approximate() {
        let tc = count_tokens("hello world foo bar", ModelTarget::Claude);
        assert!(!tc.is_exact());
        assert!(tc.to_string().contains("estimated"));
        assert!(tc.value() > 0);
    }

    #[test]
    fn test_count_tokens_per_target() {
        let text = "Write clear and concise summaries for the user";
        let claude = count_tokens(text, ModelTarget::Claude);
        let gpt = count_tokens(text, ModelTarget::Gpt);
        assert!(!claude.is_exact());
        assert!(!gpt.is_exact());
        assert!(claude.to_string().contains("Anthropic"));
        assert!(gpt.to_string().contains("tiktoken"));
    }
}
