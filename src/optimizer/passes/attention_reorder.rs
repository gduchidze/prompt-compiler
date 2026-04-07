use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct AttentionAwareReorder;

impl OptimizerPass for AttentionAwareReorder {
    fn name(&self) -> &'static str {
        "AttentionAwareReorder"
    }

    fn run(&self, mut ast: PromptAst, _ctx: &PassContext<'_>) -> PassResult {
        if ast.instructions.len() < 2 {
            return PassResult::noop(ast);
        }

        let original_order: Vec<NodeId> = ast.instructions.iter().map(|i| i.id).collect();

        // Stable sort by priority descending (Critical > High > Medium > Low)
        ast.instructions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Reorder context: high relevance first and last, low relevance in middle
        if ast.context.len() > 2 {
            let mut high_rel: Vec<ContextNode> = Vec::new();
            let mut low_rel: Vec<ContextNode> = Vec::new();

            for ctx_node in ast.context.drain(..) {
                if ctx_node.relevance_score >= 0.7 {
                    high_rel.push(ctx_node);
                } else {
                    low_rel.push(ctx_node);
                }
            }

            // High relevance at edges, low relevance in middle
            let half = high_rel.len() / 2;
            let (front, back) = high_rel.split_at(half);
            ast.context.extend(front.iter().cloned());
            ast.context.extend(low_rel);
            ast.context.extend(back.iter().cloned());
        }

        let new_order: Vec<NodeId> = ast.instructions.iter().map(|i| i.id).collect();
        let mut diagnostics = Vec::new();

        for (new_pos, new_id) in new_order.iter().enumerate() {
            let old_pos = original_order.iter().position(|id| id == new_id).unwrap();
            if old_pos != new_pos {
                diagnostics.push(PassDiagnostic::ReorderedInstruction {
                    from: old_pos,
                    to: new_pos,
                    reason: "Attention-aware ordering".into(),
                });
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
    use crate::codegen::ModelTarget;
    use crate::embedder::tfidf::TfIdfEmbedder;
    use crate::token_counter::WhitespaceCounter;

    #[test]
    fn test_critical_instructions_first() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Low priority task".into(),
            verb: String::new(), object: String::new(),
            polarity: Polarity::Positive, priority: Priority::Low,
            span: TextSpan { start: 0, end: 0 }, token_count: 3, embedding: None,
        });
        ast.instructions.push(InstructionNode {
            id: NodeId(1),
            text: "Critical task".into(),
            verb: String::new(), object: String::new(),
            polarity: Polarity::Positive, priority: Priority::Critical,
            span: TextSpan { start: 0, end: 0 }, token_count: 2, embedding: None,
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = PassContext {
            target: ModelTarget::Claude, opt_level: 2, embedder: &embedder,
            token_counter: &WhitespaceCounter, similarity_threshold: 0.85,
            context_relevance_threshold: 0.1, max_examples: 5,
        };

        let result = AttentionAwareReorder.run(ast, &ctx);
        assert_eq!(result.ast.instructions[0].priority, Priority::Critical);
        assert!(result.changes_made);
    }
}
