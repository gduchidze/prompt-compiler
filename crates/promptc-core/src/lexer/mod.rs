pub mod rules;
pub mod token;

use crate::error::CompilerError;
use crate::parser::ast::TextSpan;
use regex::Regex;
use std::sync::OnceLock;
use token::{Token, TokenKind};

static NEGATIVE_RE: OnceLock<Regex> = OnceLock::new();
static PRIORITY_RE: OnceLock<Regex> = OnceLock::new();

fn negative_regex() -> &'static Regex {
    NEGATIVE_RE.get_or_init(|| {
        Regex::new(r"(?i)\b(do\s+not|don'?t|never|avoid|refrain\s+from|without)\b").unwrap()
    })
}

fn priority_regex() -> &'static Regex {
    PRIORITY_RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(must|always|critical|important|required|ensure|guarantee|you\s+should)\b",
        )
        .unwrap()
    })
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, CompilerError> {
    let rules = rules::rules();
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < input.len() {
        let slice = &input[cursor..];

        let mut matched = false;
        for rule in rules.iter() {
            if let Some(m) = rule.regex.find(slice) {
                if m.start() == 0 && !m.as_str().is_empty() {
                    let text = m.as_str().to_string();
                    let span = TextSpan {
                        start: cursor,
                        end: cursor + m.end(),
                    };

                    // For Sentence tokens, refine the kind based on content
                    let kind = if rule.kind == TokenKind::Sentence {
                        if negative_regex().is_match(&text) && priority_regex().is_match(&text) {
                            // Has both — classify as Sentence, let parser sort it out
                            TokenKind::Sentence
                        } else if negative_regex().is_match(&text) {
                            TokenKind::Sentence // Annotated later by parser
                        } else {
                            rule.kind.clone()
                        }
                    } else {
                        rule.kind.clone()
                    };

                    tokens.push(Token { kind, text, span });

                    cursor += m.end();
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            // Skip single character
            cursor += 1;
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        span: TextSpan {
            start: cursor,
            end: cursor,
        },
    });

    Ok(tokens)
}

/// Check if text contains negative markers
pub fn has_negative_marker(text: &str) -> bool {
    negative_regex().is_match(text)
}

/// Check if text contains priority markers
pub fn has_priority_marker(text: &str) -> bool {
    priority_regex().is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_section_headers() {
        let tokens = tokenize("## Instructions\n").unwrap();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::SectionHeader));
    }

    #[test]
    fn test_lex_bullet_list() {
        let input = "- First item\n- Second item\n- Third item";
        let tokens = tokenize(input).unwrap();
        let bullets: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Bullet)
            .collect();
        assert_eq!(bullets.len(), 3);
    }

    #[test]
    fn test_lex_numbered_items() {
        let input = "1. First\n2. Second";
        let tokens = tokenize(input).unwrap();
        let numbered: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::NumberedItem)
            .collect();
        assert_eq!(numbered.len(), 2);
    }

    #[test]
    fn test_lex_example_start() {
        let input = "Input: hello world\nOutput: goodbye world";
        let tokens = tokenize(input).unwrap();
        let examples: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::ExampleStart)
            .collect();
        assert_eq!(examples.len(), 2);
    }

    #[test]
    fn test_lex_empty_string() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_has_negative_marker() {
        assert!(has_negative_marker("Do not use jargon"));
        assert!(has_negative_marker("Don't be verbose"));
        assert!(has_negative_marker("Never start with..."));
        assert!(has_negative_marker("Avoid technical terms"));
        assert!(!has_negative_marker("Write clear summaries"));
    }

    #[test]
    fn test_has_priority_marker() {
        assert!(has_priority_marker("You must always respond"));
        assert!(has_priority_marker("This is critical"));
        assert!(has_priority_marker("Ensure accuracy"));
        assert!(!has_priority_marker("Write a summary"));
    }
}
