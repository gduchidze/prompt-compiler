pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> usize;
}

pub struct WhitespaceCounter;

impl TokenCounter for WhitespaceCounter {
    fn count(&self, text: &str) -> usize {
        text.split_whitespace().count()
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
}
