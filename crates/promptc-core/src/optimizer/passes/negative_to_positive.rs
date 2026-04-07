use regex::Regex;
use std::sync::OnceLock;

use crate::codegen::ModelTarget;
use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct NegativeToPositive;

struct RewriteRule {
    pattern: Regex,
    replacement: &'static str,
}

fn rewrite_rules() -> &'static Vec<RewriteRule> {
    static RULES: OnceLock<Vec<RewriteRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            RewriteRule {
                pattern: Regex::new(r"(?i)do\s+not\s+use\s+(?:technical\s+)?jargon").unwrap(),
                replacement: "Use plain, accessible language",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)don'?t\s+use\s+(?:technical\s+)?jargon").unwrap(),
                replacement: "Use plain, accessible language",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)never\s+give\s+(?:financial|medical|legal)\s+advice")
                    .unwrap(),
                replacement: "Direct specialized questions to a qualified professional",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)do\s+not\s+(?:write|generate)\s+code").unwrap(),
                replacement: "Respond with explanations only, not code",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)don'?t\s+(?:write|generate)\s+code").unwrap(),
                replacement: "Respond with explanations only, not code",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)avoid\s+(?:being\s+)?(?:verbose|wordy)").unwrap(),
                replacement: "Be concise and direct",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)don'?t\s+(?:be\s+)?formal").unwrap(),
                replacement: "Use a conversational, relaxed tone",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)do\s+not\s+(?:be\s+)?formal").unwrap(),
                replacement: "Use a conversational, relaxed tone",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)never\s+(?:start|begin)\s+with").unwrap(),
                replacement: "Start your response with the core answer directly",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)avoid\s+repeating").unwrap(),
                replacement: "Provide unique, non-redundant content",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)do\s+not\s+include\s+(\w+)").unwrap(),
                replacement: "Exclude $1",
            },
            // Generic fallbacks — must be last
            RewriteRule {
                pattern: Regex::new(r"(?i)do\s+not\s+(.+)").unwrap(),
                replacement: "Instead of that: $1",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)don'?t\s+(.+)").unwrap(),
                replacement: "Instead of that: $1",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)never\s+(.+)").unwrap(),
                replacement: "Avoid: $1",
            },
            RewriteRule {
                pattern: Regex::new(r"(?i)avoid\s+(.+)").unwrap(),
                replacement: "Prefer alternatives to: $1",
            },
        ]
    })
}

fn try_rewrite(text: &str) -> Option<String> {
    for rule in rewrite_rules() {
        if rule.pattern.is_match(text) {
            let result = rule.pattern.replace(text, rule.replacement).to_string();
            return Some(result);
        }
    }
    None
}

impl OptimizerPass for NegativeToPositive {
    fn name(&self) -> &'static str {
        "NegativeToPositive"
    }

    fn run(&self, mut ast: PromptAst, ctx: &PassContext<'_>) -> PassResult {
        // Only run for Claude target
        if ctx.target != ModelTarget::Claude {
            return PassResult::noop(ast);
        }

        let mut diagnostics = Vec::new();

        for inst in &mut ast.instructions {
            if inst.polarity == Polarity::Negative {
                if let Some(positive) = try_rewrite(&inst.text) {
                    diagnostics.push(PassDiagnostic::ConvertedPolarity {
                        before: inst.text.clone(),
                        after: positive.clone(),
                    });
                    inst.text = positive;
                    inst.polarity = Polarity::Positive;
                }
            }
        }

        let changes_made = !diagnostics.is_empty();
        PassResult {
            ast,
            changes_made,
            diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedder::tfidf::TfIdfEmbedder;
    use crate::token_counter::WhitespaceCounter;

    fn make_ctx(target: ModelTarget, embedder: &dyn crate::embedder::Embedder) -> PassContext<'_> {
        PassContext {
            target,
            opt_level: 2,
            embedder,
            token_counter: &WhitespaceCounter,
            similarity_threshold: 0.85,
            context_relevance_threshold: 0.1,
            max_examples: 5,
        }
    }

    #[test]
    fn test_rewrite_dont_use_jargon() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Don't use jargon".into(),
            verb: "use".into(),
            object: "jargon".into(),
            polarity: Polarity::Negative,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = make_ctx(ModelTarget::Claude, &embedder);
        let result = NegativeToPositive.run(ast, &ctx);
        assert!(result.ast.instructions[0].text.contains("plain"));
        assert_eq!(result.ast.instructions[0].polarity, Polarity::Positive);
        assert!(result.changes_made);
    }

    #[test]
    fn test_preserves_positive() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Always be helpful".into(),
            verb: "be".into(),
            object: "helpful".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = make_ctx(ModelTarget::Claude, &embedder);
        let result = NegativeToPositive.run(ast, &ctx);
        assert_eq!(result.ast.instructions[0].text, "Always be helpful");
        assert!(!result.changes_made);
    }

    #[test]
    fn test_only_runs_for_claude() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Don't use jargon".into(),
            verb: "use".into(),
            object: "jargon".into(),
            polarity: Polarity::Negative,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = make_ctx(ModelTarget::Gpt, &embedder);
        let result = NegativeToPositive.run(ast, &ctx);
        assert_eq!(result.ast.instructions[0].text, "Don't use jargon");
        assert!(!result.changes_made);
    }
}
