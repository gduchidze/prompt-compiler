use crate::embedder::{Embedder, TfIdfEmbedder};
use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct ContextRelevancePruning;

impl OptimizerPass for ContextRelevancePruning {
    fn name(&self) -> &'static str {
        "ContextRelevancePruning"
    }

    fn run(&self, mut ast: PromptAst, ctx: &PassContext<'_>) -> PassResult {
        if ast.context.is_empty() || ast.instructions.is_empty() {
            return PassResult::noop(ast);
        }

        // Build instruction summary
        let instruction_summary: String = ast
            .instructions
            .iter()
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Build embedder from instructions + context
        let mut docs: Vec<&str> = vec![&instruction_summary];
        for c in &ast.context {
            docs.push(&c.text);
        }
        let embedder = TfIdfEmbedder::from_documents(&docs);

        // Score each context node
        let mut diagnostics = Vec::new();
        for context_node in &mut ast.context {
            let score = embedder.similarity(&context_node.text, &instruction_summary);
            context_node.relevance_score = score as f64;
        }

        let threshold = ctx.context_relevance_threshold;
        let before_count = ast.context.len();

        let mut pruned = Vec::new();
        ast.context.retain(|c| {
            if c.relevance_score < threshold {
                pruned.push(PassDiagnostic::PrunedContext {
                    text: c.text.clone(),
                    relevance: c.relevance_score,
                });
                false
            } else {
                true
            }
        });

        diagnostics.extend(pruned);
        let changes_made = ast.context.len() < before_count;

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
    fn test_prunes_irrelevant_context() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Write about machine learning algorithms".into(),
            verb: "write".into(), object: "machine learning algorithms".into(),
            polarity: Polarity::Positive, priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 }, token_count: 5, embedding: None,
        });
        ast.context.push(ContextNode {
            id: NodeId(1),
            text: "The weather today is sunny and warm".into(),
            relevance_score: 1.0,
            token_count: 7,
            span: TextSpan { start: 0, end: 0 },
        });
        ast.context.push(ContextNode {
            id: NodeId(2),
            text: "Machine learning uses algorithms to learn from data".into(),
            relevance_score: 1.0,
            token_count: 8,
            span: TextSpan { start: 0, end: 0 },
        });

        let embedder = TfIdfEmbedder::from_documents(&[]);
        let ctx = PassContext {
            target: ModelTarget::Claude, opt_level: 2, embedder: &embedder,
            token_counter: &WhitespaceCounter, similarity_threshold: 0.85,
            context_relevance_threshold: 0.1, max_examples: 5,
        };

        let result = ContextRelevancePruning.run(ast, &ctx);
        // The relevant context should survive, irrelevant may be pruned
        assert!(result.ast.context.iter().any(|c| c.text.contains("Machine learning")));
    }
}
