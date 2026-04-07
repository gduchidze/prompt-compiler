use regex::Regex;
use std::sync::OnceLock;

use crate::parser::ast::TextSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct GptismFinding {
    pub pattern: &'static str,
    pub span: TextSpan,
    pub found: String,
    pub suggestion: String,
    pub severity: Severity,
}

struct GptismRule {
    pattern: Regex,
    description: &'static str,
    suggestion: &'static str,
    severity: Severity,
}

fn rules() -> &'static Vec<GptismRule> {
    static RULES: OnceLock<Vec<GptismRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            GptismRule {
                pattern: Regex::new(r"(?i)let'?s\s+think\s+step[\s-]+by[\s-]+step").unwrap(),
                description: "Chain-of-thought prompt (GPT-style)",
                suggestion: "Use <thinking> tags or 'Think through this step-by-step:' for Claude",
                severity: Severity::Warning,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)as\s+an?\s+ai\s+(?:language\s+)?model").unwrap(),
                description: "AI self-reference (GPT-style)",
                suggestion: "Remove entirely — Claude doesn't need this preamble",
                severity: Severity::Warning,
            },
            GptismRule {
                pattern: Regex::new(r"(?m)^###\s+\w").unwrap(),
                description: "Markdown ### headers",
                suggestion: "Convert to XML tags (e.g., <section_name>) for Claude",
                severity: Severity::Info,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)\bgpt[-_]?4\b").unwrap(),
                description: "GPT-4 model reference",
                suggestion: "Remove model-specific reference",
                severity: Severity::Warning,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)\bchatgpt\b").unwrap(),
                description: "ChatGPT reference",
                suggestion: "Remove model-specific reference",
                severity: Severity::Warning,
            },
            GptismRule {
                pattern: Regex::new(r"\*\*[^*]+\*\*").unwrap(),
                description: "Markdown bold formatting",
                suggestion: "Consider using <important>...</important> XML tags for Claude",
                severity: Severity::Info,
            },
            GptismRule {
                pattern: Regex::new(r"(?:^|[^*])\*[^*]+\*(?:[^*]|$)").unwrap(),
                description: "Markdown italic formatting",
                suggestion: "Use plain text or XML emphasis for Claude",
                severity: Severity::Info,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)\bcertainly[,!]").unwrap(),
                description: "Sycophantic opener",
                suggestion: "Remove — encourages sycophantic responses",
                severity: Severity::Info,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)\bof\s+course[,!]").unwrap(),
                description: "Sycophantic opener",
                suggestion: "Remove — encourages sycophantic responses",
                severity: Severity::Info,
            },
            GptismRule {
                pattern: Regex::new(r"(?i)in\s+summary[,:]").unwrap(),
                description: "Redundant summary marker",
                suggestion: "Often redundant — consider removing",
                severity: Severity::Info,
            },
        ]
    })
}

pub fn detect_gptisms(text: &str) -> Vec<GptismFinding> {
    let mut findings = Vec::new();

    for rule in rules() {
        for m in rule.pattern.find_iter(text) {
            findings.push(GptismFinding {
                pattern: rule.description,
                span: TextSpan {
                    start: m.start(),
                    end: m.end(),
                },
                found: m.as_str().to_string(),
                suggestion: rule.suggestion.to_string(),
                severity: rule.severity.clone(),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_lets_think_step_by_step() {
        let findings = detect_gptisms("Let's think step by step about this problem.");
        assert!(!findings.is_empty());
        assert!(findings[0].pattern.contains("Chain-of-thought"));
    }

    #[test]
    fn test_detects_as_an_ai_model() {
        let findings = detect_gptisms("As an AI language model, I can help you.");
        assert!(!findings.is_empty());
        assert!(findings[0].pattern.contains("AI self-reference"));
    }

    #[test]
    fn test_detects_markdown_headers() {
        let findings = detect_gptisms("### Instructions\nDo something.");
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_detects_gpt4_reference() {
        let findings = detect_gptisms("You are GPT-4, a large language model.");
        assert!(!findings.is_empty());
        assert!(findings[0].pattern.contains("GPT-4"));
    }

    #[test]
    fn test_detects_chatgpt_reference() {
        let findings = detect_gptisms("You are ChatGPT.");
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_clean_prompt_no_findings() {
        let findings = detect_gptisms(
            "You are a helpful assistant. Provide clear, concise answers to user questions.",
        );
        assert!(
            findings.is_empty(),
            "Unexpected findings: {:?}",
            findings.iter().map(|f| &f.found).collect::<Vec<_>>()
        );
    }
}
