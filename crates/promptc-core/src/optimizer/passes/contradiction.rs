use std::collections::HashMap;

use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct ContradictionResolver;

fn normalize_object(obj: &str) -> String {
    obj.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl OptimizerPass for ContradictionResolver {
    fn name(&self) -> &'static str {
        "ContradictionResolver"
    }

    fn run(&self, mut ast: PromptAst, _ctx: &PassContext<'_>) -> PassResult {
        if ast.instructions.len() < 2 {
            return PassResult::noop(ast);
        }

        let mut diagnostics = Vec::new();
        let mut to_remove = Vec::new();

        // Group instructions by normalized object
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, inst) in ast.instructions.iter().enumerate() {
            let key = normalize_object(&inst.object);
            if !key.is_empty() {
                groups.entry(key).or_default().push(idx);
            }
        }

        for indices in groups.values() {
            if indices.len() < 2 {
                continue;
            }
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let a = &ast.instructions[indices[i]];
                    let b = &ast.instructions[indices[j]];

                    if a.polarity != b.polarity {
                        diagnostics.push(PassDiagnostic::ContradictionFound {
                            text_a: a.text.clone(),
                            text_b: b.text.clone(),
                        });

                        // Remove lower priority; if equal, remove negative
                        let remove_idx = if a.priority > b.priority {
                            indices[j]
                        } else if b.priority > a.priority {
                            indices[i]
                        } else if a.polarity == Polarity::Positive {
                            indices[j]
                        } else {
                            indices[i]
                        };

                        let keep_idx = if remove_idx == indices[i] {
                            indices[j]
                        } else {
                            indices[i]
                        };

                        diagnostics.push(PassDiagnostic::ContradictionResolved {
                            kept: ast.instructions[keep_idx].text.clone(),
                            removed: ast.instructions[remove_idx].text.clone(),
                        });

                        to_remove.push(ast.instructions[remove_idx].id);
                    }
                }
            }
        }

        let changes_made = !to_remove.is_empty();
        ast.instructions.retain(|i| !to_remove.contains(&i.id));

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
    use crate::codegen::ModelTarget;
    use crate::embedder::tfidf::TfIdfEmbedder;
    use crate::token_counter::WhitespaceCounter;

    fn make_ctx(embedder: &dyn crate::embedder::Embedder) -> PassContext<'_> {
        PassContext {
            target: ModelTarget::Claude,
            opt_level: 2,
            embedder,
            token_counter: &WhitespaceCounter,
            similarity_threshold: 0.85,
            context_relevance_threshold: 0.1,
            max_examples: 5,
        }
    }

    #[test]
    fn test_detects_contradiction() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Use formal tone".into(),
            verb: "use".into(),
            object: "formal tone".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });
        ast.instructions.push(InstructionNode {
            id: NodeId(1),
            text: "Don't be formal tone".into(),
            verb: "be".into(),
            object: "formal tone".into(),
            polarity: Polarity::Negative,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 4,
            embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = make_ctx(&embedder);
        let result = ContradictionResolver.run(ast, &ctx);
        assert!(result.changes_made);
        assert_eq!(result.ast.instructions.len(), 1);
        // Should keep the positive one when priority is equal
        assert_eq!(result.ast.instructions[0].polarity, Polarity::Positive);
    }

    #[test]
    fn test_keeps_higher_priority() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Use formal tone".into(),
            verb: "use".into(),
            object: "formal tone".into(),
            polarity: Polarity::Positive,
            priority: Priority::Low,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });
        ast.instructions.push(InstructionNode {
            id: NodeId(1),
            text: "Never use formal tone".into(),
            verb: "use".into(),
            object: "formal tone".into(),
            polarity: Polarity::Negative,
            priority: Priority::Critical,
            span: TextSpan { start: 0, end: 0 },
            token_count: 4,
            embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = make_ctx(&embedder);
        let result = ContradictionResolver.run(ast, &ctx);
        assert!(result.changes_made);
        assert_eq!(result.ast.instructions.len(), 1);
        assert_eq!(result.ast.instructions[0].priority, Priority::Critical);
    }
}
