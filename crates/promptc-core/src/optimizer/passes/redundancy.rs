use std::collections::HashMap;

use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct RedundancyElimination;

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl OptimizerPass for RedundancyElimination {
    fn name(&self) -> &'static str {
        "RedundancyElimination"
    }

    fn run(&self, mut ast: PromptAst, _ctx: &PassContext<'_>) -> PassResult {
        let mut diagnostics = Vec::new();

        // Deduplicate instructions by normalized text
        {
            let mut seen: HashMap<String, (usize, Priority)> = HashMap::new();
            let mut to_remove = Vec::new();

            for (idx, inst) in ast.instructions.iter().enumerate() {
                let norm = normalize_text(&inst.text);
                if let Some((prev_idx, prev_priority)) = seen.get(&norm) {
                    // Keep the one with higher priority
                    if inst.priority > *prev_priority {
                        to_remove.push(*prev_idx);
                        seen.insert(norm, (idx, inst.priority));
                    } else {
                        to_remove.push(idx);
                    }
                    diagnostics.push(PassDiagnostic::RemovedInstruction {
                        text: inst.text.clone(),
                        reason: "Exact text duplicate".into(),
                    });
                } else {
                    seen.insert(norm, (idx, inst.priority));
                }
            }

            let to_remove_ids: Vec<NodeId> = to_remove
                .iter()
                .map(|&idx| ast.instructions[idx].id)
                .collect();
            ast.instructions.retain(|i| !to_remove_ids.contains(&i.id));
        }

        // Deduplicate context by normalized text
        {
            let mut seen: HashMap<String, NodeId> = HashMap::new();
            let mut to_remove = Vec::new();

            for ctx_node in &ast.context {
                let norm = normalize_text(&ctx_node.text);
                if seen.contains_key(&norm) {
                    to_remove.push(ctx_node.id);
                    diagnostics.push(PassDiagnostic::PrunedContext {
                        text: ctx_node.text.clone(),
                        relevance: ctx_node.relevance_score,
                    });
                } else {
                    seen.insert(norm, ctx_node.id);
                }
            }

            ast.context.retain(|c| !to_remove.contains(&c.id));
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
    use crate::codegen::ModelTarget;
    use crate::embedder::tfidf::TfIdfEmbedder;
    use crate::token_counter::WhitespaceCounter;

    #[test]
    fn test_removes_exact_duplicates() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Write clear summaries.".into(),
            verb: "write".into(), object: "clear summaries".into(),
            polarity: Polarity::Positive, priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 }, token_count: 3, embedding: None,
        });
        ast.instructions.push(InstructionNode {
            id: NodeId(1),
            text: "Write clear summaries.".into(),
            verb: "write".into(), object: "clear summaries".into(),
            polarity: Polarity::Positive, priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 }, token_count: 3, embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = PassContext {
            target: ModelTarget::Claude, opt_level: 2, embedder: &embedder,
            token_counter: &WhitespaceCounter, similarity_threshold: 0.85,
            context_relevance_threshold: 0.1, max_examples: 5,
        };

        let result = RedundancyElimination.run(ast, &ctx);
        assert_eq!(result.ast.instructions.len(), 1);
        assert!(result.changes_made);
    }
}
