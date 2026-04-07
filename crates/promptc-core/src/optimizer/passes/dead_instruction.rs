use std::collections::HashSet;

use crate::embedder::{Embedder, TfIdfEmbedder};
use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct DeadInstructionElimination;

impl OptimizerPass for DeadInstructionElimination {
    fn name(&self) -> &'static str {
        "DeadInstructionElimination"
    }

    fn run(&self, mut ast: PromptAst, ctx: &PassContext<'_>) -> PassResult {
        if ast.instructions.len() < 2 {
            return PassResult::noop(ast);
        }

        let docs: Vec<&str> = ast.instructions.iter().map(|i| i.text.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&docs);

        let embeddings: Vec<Vec<f32>> = ast
            .instructions
            .iter()
            .map(|i| embedder.embed(&i.text))
            .collect();

        let mut dead: HashSet<NodeId> = HashSet::new();
        let mut diagnostics = Vec::new();

        for i in 0..embeddings.len() {
            if dead.contains(&ast.instructions[i].id) {
                continue;
            }
            for j in (i + 1)..embeddings.len() {
                if dead.contains(&ast.instructions[j].id) {
                    continue;
                }
                let sim = crate::embedder::cosine_similarity(&embeddings[i], &embeddings[j]);
                if sim > ctx.similarity_threshold {
                    // Keep higher priority; if equal, keep shorter
                    let remove = if ast.instructions[i].priority > ast.instructions[j].priority {
                        j
                    } else if ast.instructions[j].priority > ast.instructions[i].priority {
                        i
                    } else if ast.instructions[i].token_count <= ast.instructions[j].token_count {
                        j
                    } else {
                        i
                    };

                    let keep = if remove == i { j } else { i };
                    dead.insert(ast.instructions[remove].id);
                    diagnostics.push(PassDiagnostic::RemovedInstruction {
                        text: ast.instructions[remove].text.clone(),
                        reason: format!(
                            "Redundant with '{}' (similarity={:.2})",
                            ast.instructions[keep].text, sim
                        ),
                    });
                }
            }
        }

        let changes_made = !dead.is_empty();
        ast.instructions.retain(|i| !dead.contains(&i.id));

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

    fn make_instruction(id: u32, text: &str, priority: Priority) -> InstructionNode {
        InstructionNode {
            id: NodeId(id),
            text: text.to_string(),
            verb: String::new(),
            object: text.to_string(),
            polarity: Polarity::Positive,
            priority,
            span: crate::parser::ast::TextSpan { start: 0, end: 0 },
            token_count: text.split_whitespace().count(),
            embedding: None,
        }
    }

    #[test]
    fn test_removes_near_duplicate() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(make_instruction(
            0,
            "Write clear and concise summaries of the content",
            Priority::Medium,
        ));
        ast.instructions.push(make_instruction(
            1,
            "Write concise and clear summaries of the content",
            Priority::Medium,
        ));
        ast.instructions.push(make_instruction(
            2,
            "Explain quantum physics in detail",
            Priority::Medium,
        ));

        let docs: Vec<&str> = ast.instructions.iter().map(|i| i.text.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&docs);
        let ctx = make_ctx(&embedder);

        let result = DeadInstructionElimination.run(ast, &ctx);
        // The two similar instructions should be reduced
        assert!(
            result.ast.instructions.len() <= 2,
            "got {}",
            result.ast.instructions.len()
        );
        assert!(result.changes_made);
    }

    #[test]
    fn test_keeps_dissimilar() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(make_instruction(
            0,
            "Write clear summaries about cats",
            Priority::Medium,
        ));
        ast.instructions.push(make_instruction(
            1,
            "Explain quantum physics thoroughly",
            Priority::Medium,
        ));

        let docs: Vec<&str> = ast.instructions.iter().map(|i| i.text.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&docs);
        let ctx = make_ctx(&embedder);

        let result = DeadInstructionElimination.run(ast, &ctx);
        assert_eq!(result.ast.instructions.len(), 2);
        assert!(!result.changes_made);
    }

    #[test]
    fn test_priority_tiebreak() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(make_instruction(
            0,
            "Write clear concise summaries of content",
            Priority::High,
        ));
        ast.instructions.push(make_instruction(
            1,
            "Write concise clear summaries of content",
            Priority::Medium,
        ));

        let docs: Vec<&str> = ast.instructions.iter().map(|i| i.text.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&docs);
        let ctx = make_ctx(&embedder);

        let result = DeadInstructionElimination.run(ast, &ctx);
        if result.changes_made {
            assert_eq!(result.ast.instructions.len(), 1);
            assert_eq!(result.ast.instructions[0].priority, Priority::High);
        }
    }
}
