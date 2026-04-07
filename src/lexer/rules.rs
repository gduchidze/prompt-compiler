use regex::Regex;
use std::sync::OnceLock;

use super::token::TokenKind;

pub struct LexerRule {
    pub kind: TokenKind,
    pub regex: Regex,
}

fn build_rules() -> Vec<LexerRule> {
    vec![
        // 1. Section headers: ## Title, [SECTION], or bare keyword headers
        LexerRule {
            kind: TokenKind::SectionHeader,
            regex: Regex::new(r"(?mi)^(?:#{1,4}\s+\S.*|(?:PERSONA|INSTRUCTIONS|CONSTRAINTS|CONTEXT|EXAMPLES|FORMAT|RULES)[\s:]*.*|\[[\w\s]+\].*)$").unwrap(),
        },
        // 2. Numbered items: 1. text or 1) text
        LexerRule {
            kind: TokenKind::NumberedItem,
            regex: Regex::new(r"(?m)^\s*\d+[.)]\s+\S[^\n]*").unwrap(),
        },
        // 3. Bullet points
        LexerRule {
            kind: TokenKind::Bullet,
            regex: Regex::new(r"(?m)^\s*[-*•]\s+\S[^\n]*").unwrap(),
        },
        // 4. Example start markers
        LexerRule {
            kind: TokenKind::ExampleStart,
            regex: Regex::new(r"(?mi)^(?:example\s*\d*|input|output|user|assistant)\s*:[^\n]*").unwrap(),
        },
        // 5. Format directives
        LexerRule {
            kind: TokenKind::FormatDirective,
            regex: Regex::new(r"(?i)(?:respond\s+in|output\s+format|format\s*:)\s*[^\n]*").unwrap(),
        },
        // 6. Sentence fallback: any non-empty line not matched above
        LexerRule {
            kind: TokenKind::Sentence,
            regex: Regex::new(r"(?m)^[^\n]+").unwrap(),
        },
        // 7. Whitespace: blank lines
        LexerRule {
            kind: TokenKind::Whitespace,
            regex: Regex::new(r"\n+|\r\n+").unwrap(),
        },
    ]
}

static RULES: OnceLock<Vec<LexerRule>> = OnceLock::new();

pub fn rules() -> &'static Vec<LexerRule> {
    RULES.get_or_init(build_rules)
}
