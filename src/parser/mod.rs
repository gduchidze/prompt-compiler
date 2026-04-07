pub mod ast;

use ast::*;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

use crate::error::CompilerError;
use crate::lexer::token::{Token, TokenKind};
use crate::token_counter::{TokenCounter, WhitespaceCounter};
use regex::Regex;

const ACTION_VERBS: &[&str] = &[
    "write", "generate", "create", "produce", "analyze", "summarize", "explain", "describe",
    "list", "extract", "translate", "classify", "identify", "compare", "evaluate", "suggest",
    "provide", "return", "output", "format", "avoid", "do", "use", "include", "exclude", "ensure",
    "maintain", "follow", "apply", "consider", "respond", "give", "make", "keep", "check",
    "review", "act", "be", "think", "speak", "answer", "reply", "help",
];

static NEGATIVE_RE: OnceLock<Regex> = OnceLock::new();

fn negative_regex() -> &'static Regex {
    NEGATIVE_RE
        .get_or_init(|| Regex::new(r"(?i)\b(do\s+not|don'?t|never|avoid|refrain\s+from|without|not)\b").unwrap())
}

#[derive(Debug, Clone, PartialEq)]
enum SectionKind {
    Persona,
    Instructions,
    Constraints,
    Context,
    Examples,
    Format,
    Unknown,
}

fn classify_section_header(text: &str) -> SectionKind {
    let lower = text.to_lowercase();
    let lower = lower.trim_start_matches('#').trim();
    let lower = lower.trim_start_matches('[').trim_end_matches(']').trim();

    if lower.starts_with("persona") || lower.starts_with("role") || lower.starts_with("identity")
    {
        SectionKind::Persona
    } else if lower.starts_with("instruction") || lower.starts_with("task") || lower.starts_with("objective") {
        SectionKind::Instructions
    } else if lower.starts_with("constraint") || lower.starts_with("rule") || lower.starts_with("limitation") {
        SectionKind::Constraints
    } else if lower.starts_with("context") || lower.starts_with("background") || lower.starts_with("information") {
        SectionKind::Context
    } else if lower.starts_with("example") {
        SectionKind::Examples
    } else if lower.starts_with("format") || lower.starts_with("output") {
        SectionKind::Format
    } else {
        SectionKind::Unknown
    }
}

fn detect_polarity(text: &str) -> Polarity {
    if negative_regex().is_match(text) {
        Polarity::Negative
    } else {
        Polarity::Positive
    }
}

fn detect_priority(text: &str) -> Priority {
    let lower = text.to_lowercase();
    let critical_words = ["must", "critical", "always", "required", "guarantee", "never"];
    let high_words = ["important", "ensure", "should", "strongly"];
    let low_words = ["optionally", "if possible", "when convenient", "feel free"];

    for word in &critical_words {
        if lower.contains(word) {
            return Priority::Critical;
        }
    }
    for word in &high_words {
        if lower.contains(word) {
            return Priority::High;
        }
    }
    for word in &low_words {
        if lower.contains(word) {
            return Priority::Low;
        }
    }
    Priority::Medium
}

fn extract_verb_object(text: &str) -> (String, String) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let limit = words.len().min(8);

    for (i, word) in words[..limit].iter().enumerate() {
        let lower = word.to_lowercase();
        let clean = lower.trim_matches(|c: char| !c.is_alphabetic());
        if ACTION_VERBS.contains(&clean) {
            let object = words[i + 1..].join(" ");
            let object = object.trim_end_matches(|c: char| c == '.' || c == ',' || c == '!');
            return (clean.to_string(), object.to_string());
        }
    }

    (String::new(), text.to_string())
}

fn detect_format_type(text: &str) -> FormatType {
    let lower = text.to_lowercase();
    if lower.contains("json") {
        FormatType::Json
    } else if lower.contains("xml") {
        FormatType::Xml
    } else if lower.contains("markdown") || lower.contains("md") {
        FormatType::Markdown
    } else if lower.contains("csv") {
        FormatType::Csv
    } else if lower.contains("list") {
        FormatType::List
    } else if lower.contains("table") {
        FormatType::Table
    } else {
        FormatType::PlainText
    }
}

struct ParserState {
    next_id: u32,
    counter: Box<dyn TokenCounter>,
    warnings: Vec<Warning>,
}

impl ParserState {
    fn new() -> Self {
        Self {
            next_id: 0,
            counter: Box::new(WhitespaceCounter),
            warnings: Vec::new(),
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

pub fn parse(tokens: Vec<Token>, source: &str) -> Result<PromptAst, CompilerError> {
    let hash = format!("{:x}", Sha256::digest(source.as_bytes()));
    let mut state = ParserState::new();

    // Group tokens into sections
    let mut sections: Vec<(SectionKind, Vec<&Token>)> = Vec::new();
    let mut current_section = SectionKind::Instructions; // default section
    let mut current_tokens: Vec<&Token> = Vec::new();
    let mut has_explicit_sections = false;

    for token in &tokens {
        match &token.kind {
            TokenKind::SectionHeader => {
                has_explicit_sections = true;
                if !current_tokens.is_empty() {
                    sections.push((current_section.clone(), current_tokens));
                    current_tokens = Vec::new();
                }
                current_section = classify_section_header(&token.text);
            }
            TokenKind::Eof => {}
            TokenKind::Whitespace => {}
            _ => {
                current_tokens.push(token);
            }
        }
    }
    if !current_tokens.is_empty() {
        sections.push((current_section, current_tokens));
    }

    // If no explicit sections, heuristically classify each token
    if !has_explicit_sections && sections.len() == 1 {
        let (_, all_tokens) = sections.remove(0);
        sections = heuristic_classify(all_tokens);
    }

    // Build AST
    let mut ast = PromptAst::empty(hash);

    for (kind, section_tokens) in &sections {
        match kind {
            SectionKind::Persona => {
                let text = section_tokens
                    .iter()
                    .map(|t| t.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ");
                let span = span_of(section_tokens);

                // Extract role: first sentence or up to first period
                let role = text
                    .split('.')
                    .next()
                    .unwrap_or(&text)
                    .trim_start_matches("You are ")
                    .trim_start_matches("you are ")
                    .trim()
                    .to_string();
                let attributes: Vec<String> = text
                    .split('.')
                    .skip(1)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                ast.persona = Some(PersonaNode {
                    id: state.alloc_id(),
                    text: text.clone(),
                    role,
                    attributes,
                    span,
                });
            }
            SectionKind::Instructions => {
                for token in section_tokens {
                    let text = clean_text(&token.text);
                    if text.is_empty() {
                        continue;
                    }
                    let (verb, object) = extract_verb_object(&text);
                    let polarity = detect_polarity(&text);
                    let priority = detect_priority(&text);
                    let token_count = state.counter.count(&text);

                    ast.instructions.push(InstructionNode {
                        id: state.alloc_id(),
                        text,
                        verb,
                        object,
                        polarity,
                        priority,
                        span: token.span,
                        token_count,
                        embedding: None,
                    });
                }
            }
            SectionKind::Constraints => {
                for token in section_tokens {
                    let text = clean_text(&token.text);
                    if text.is_empty() {
                        continue;
                    }
                    let priority = detect_priority(&text);
                    let token_count = state.counter.count(&text);

                    ast.constraints.push(ConstraintNode {
                        id: state.alloc_id(),
                        text,
                        priority,
                        span: token.span,
                        token_count,
                    });
                }
            }
            SectionKind::Context => {
                for token in section_tokens {
                    let text = clean_text(&token.text);
                    if text.is_empty() {
                        continue;
                    }
                    let token_count = state.counter.count(&text);

                    ast.context.push(ContextNode {
                        id: state.alloc_id(),
                        text,
                        relevance_score: 1.0,
                        token_count,
                        span: token.span,
                    });
                }
            }
            SectionKind::Examples => {
                parse_examples(section_tokens, &mut ast, &mut state);
            }
            SectionKind::Format => {
                let text = section_tokens
                    .iter()
                    .map(|t| t.text.trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                let span = span_of(section_tokens);
                let format_type = detect_format_type(&text);

                ast.format_spec = Some(FormatNode {
                    id: state.alloc_id(),
                    text: clean_text(&text),
                    format_type,
                    span,
                });
            }
            SectionKind::Unknown => {
                // Treat as instructions
                for token in section_tokens {
                    let text = clean_text(&token.text);
                    if text.is_empty() {
                        continue;
                    }
                    let (verb, object) = extract_verb_object(&text);
                    let polarity = detect_polarity(&text);
                    let priority = detect_priority(&text);
                    let token_count = state.counter.count(&text);

                    ast.instructions.push(InstructionNode {
                        id: state.alloc_id(),
                        text,
                        verb,
                        object,
                        polarity,
                        priority,
                        span: token.span,
                        token_count,
                        embedding: None,
                    });
                }
            }
        }
    }

    // Compute total tokens
    let total: usize = ast.instructions.iter().map(|n| n.token_count).sum::<usize>()
        + ast.constraints.iter().map(|n| n.token_count).sum::<usize>()
        + ast.context.iter().map(|n| n.token_count).sum::<usize>()
        + ast.examples.iter().map(|n| n.token_count).sum::<usize>()
        + ast.persona.as_ref().map(|p| state.counter.count(&p.text)).unwrap_or(0)
        + ast.format_spec.as_ref().map(|f| state.counter.count(&f.text)).unwrap_or(0);

    ast.metadata.total_tokens = total;
    ast.metadata.parse_warnings = state.warnings;

    Ok(ast)
}

fn parse_examples(tokens: &[&Token], ast: &mut PromptAst, state: &mut ParserState) {
    let mut input: Option<String> = None;

    for token in tokens {
        let text = token.text.trim();
        let lower = text.to_lowercase();

        if lower.starts_with("input:") || lower.starts_with("user:") {
            let content = text.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            input = Some(content);
        } else if (lower.starts_with("output:") || lower.starts_with("assistant:"))
            && input.is_some()
        {
            let content = text.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            let inp = input.take().unwrap();
            let token_count = state.counter.count(&inp) + state.counter.count(&content);

            // Simple demonstrates extraction
            let demonstrates = extract_demonstrates(&inp, &content);

            ast.examples.push(ExampleNode {
                id: state.alloc_id(),
                input: inp,
                output: content,
                demonstrates,
                diversity_score: 0.0,
                token_count,
            });
        } else if input.is_none() && token.kind == TokenKind::ExampleStart {
            // example label like "Example 1:" — check if it has content after colon
            if let Some(content) = text.splitn(2, ':').nth(1) {
                let content = content.trim();
                if !content.is_empty() {
                    input = Some(content.to_string());
                }
            }
        }
    }
}

fn extract_demonstrates(_input: &str, _output: &str) -> Vec<String> {
    // Simple heuristic: look for common demonstration categories
    let mut cats = Vec::new();
    let combined = format!("{_input} {_output}").to_lowercase();

    if combined.contains("format") || combined.contains("json") || combined.contains("xml") {
        cats.push("format".to_string());
    }
    if combined.contains("tone") || combined.contains("formal") || combined.contains("casual") {
        cats.push("tone".to_string());
    }
    if combined.contains("reason") || combined.contains("because") || combined.contains("therefore")
    {
        cats.push("reasoning".to_string());
    }
    if combined.contains("step") || combined.contains("first") || combined.contains("then") {
        cats.push("process".to_string());
    }

    if cats.is_empty() {
        cats.push("general".to_string());
    }

    cats
}

fn heuristic_classify(tokens: Vec<&Token>) -> Vec<(SectionKind, Vec<&Token>)> {
    let mut instructions = Vec::new();
    let mut examples = Vec::new();
    let mut format_tokens = Vec::new();

    for token in tokens {
        match &token.kind {
            TokenKind::ExampleStart => examples.push(token),
            TokenKind::FormatDirective => format_tokens.push(token),
            _ => instructions.push(token),
        }
    }

    let mut sections = Vec::new();
    if !instructions.is_empty() {
        sections.push((SectionKind::Instructions, instructions));
    }
    if !examples.is_empty() {
        sections.push((SectionKind::Examples, examples));
    }
    if !format_tokens.is_empty() {
        sections.push((SectionKind::Format, format_tokens));
    }
    sections
}

fn clean_text(text: &str) -> String {
    let trimmed = text.trim();
    let trimmed = trimmed.trim_start_matches(|c: char| c == '-' || c == '*' || c == '•');
    let trimmed = trimmed.trim();
    // Strip leading numbered list markers
    let trimmed = if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        rest.trim_start_matches(|c: char| c.is_ascii_digit())
            .trim_start_matches('.')
            .trim_start_matches(')')
            .trim()
    } else {
        trimmed
    };
    trimmed.to_string()
}

fn span_of(tokens: &[&Token]) -> TextSpan {
    if tokens.is_empty() {
        return TextSpan { start: 0, end: 0 };
    }
    TextSpan {
        start: tokens.first().unwrap().span.start,
        end: tokens.last().unwrap().span.end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    #[test]
    fn test_parse_instruction_polarity_positive() {
        let tokens = lexer::tokenize("## Instructions\nWrite clear summaries.").unwrap();
        let ast = parse(tokens, "Write clear summaries.").unwrap();
        assert!(!ast.instructions.is_empty());
        assert_eq!(ast.instructions[0].polarity, Polarity::Positive);
    }

    #[test]
    fn test_parse_instruction_polarity_negative() {
        let tokens = lexer::tokenize("## Instructions\n- Do not use jargon.").unwrap();
        let ast = parse(tokens, "Do not use jargon.").unwrap();
        assert!(!ast.instructions.is_empty());
        assert_eq!(ast.instructions[0].polarity, Polarity::Negative);
    }

    #[test]
    fn test_parse_priority_critical() {
        let tokens =
            lexer::tokenize("## Instructions\nYou must always respond accurately.").unwrap();
        let ast = parse(tokens, "must always").unwrap();
        assert!(!ast.instructions.is_empty());
        assert_eq!(ast.instructions[0].priority, Priority::Critical);
    }

    #[test]
    fn test_parse_sections() {
        let input = "## Persona\nYou are a helpful assistant.\n\n## Instructions\n- Write clearly.\n- Be concise.";
        let tokens = lexer::tokenize(input).unwrap();
        let ast = parse(tokens, input).unwrap();
        assert!(ast.persona.is_some());
        assert!(!ast.instructions.is_empty());
    }

    #[test]
    fn test_parse_metadata_hash() {
        let t1 = lexer::tokenize("## Instructions\nFirst").unwrap();
        let t2 = lexer::tokenize("## Instructions\nSecond").unwrap();
        let ast1 = parse(t1, "First").unwrap();
        let ast2 = parse(t2, "Second").unwrap();
        assert_ne!(ast1.metadata.source_hash, ast2.metadata.source_hash);
    }

    #[test]
    fn test_parse_examples() {
        let input = "## Examples\nInput: hello\nOutput: world";
        let tokens = lexer::tokenize(input).unwrap();
        let ast = parse(tokens, input).unwrap();
        assert_eq!(ast.examples.len(), 1);
        assert_eq!(ast.examples[0].input, "hello");
        assert_eq!(ast.examples[0].output, "world");
    }

    #[test]
    fn test_verb_extraction() {
        let (verb, object) = extract_verb_object("Write clear and concise summaries.");
        assert_eq!(verb, "write");
        assert!(!object.is_empty());
    }
}
