use crate::embedder::{cosine_similarity, Embedder, TfIdfEmbedder};
use crate::optimizer::pass::*;
use crate::parser::ast::*;

pub struct ExampleDiversitySelection;

impl OptimizerPass for ExampleDiversitySelection {
    fn name(&self) -> &'static str {
        "ExampleDiversitySelection"
    }

    fn run(&self, mut ast: PromptAst, ctx: &PassContext<'_>) -> PassResult {
        let max = ctx.max_examples;
        if ast.examples.len() <= max {
            return PassResult::noop(ast);
        }

        let docs: Vec<&str> = ast.examples.iter().map(|e| e.input.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&docs);
        let embeddings: Vec<Vec<f32>> = docs.iter().map(|d| embedder.embed(d)).collect();

        // Greedy max-coverage selection
        let n = embeddings.len();
        let mut selected: Vec<usize> = Vec::new();
        let mut remaining: Vec<usize> = (0..n).collect();

        // Start with the most "unique" example (lowest average similarity)
        let first = remaining
            .iter()
            .copied()
            .min_by(|&a, &b| {
                let avg_a: f32 = (0..n)
                    .filter(|&j| j != a)
                    .map(|j| cosine_similarity(&embeddings[a], &embeddings[j]))
                    .sum::<f32>()
                    / (n - 1).max(1) as f32;
                let avg_b: f32 = (0..n)
                    .filter(|&j| j != b)
                    .map(|j| cosine_similarity(&embeddings[b], &embeddings[j]))
                    .sum::<f32>()
                    / (n - 1).max(1) as f32;
                avg_a
                    .partial_cmp(&avg_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        selected.push(first);
        remaining.retain(|&x| x != first);

        while selected.len() < max && !remaining.is_empty() {
            let best = remaining
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let min_dist_a: f32 = selected
                        .iter()
                        .map(|&s| 1.0 - cosine_similarity(&embeddings[a], &embeddings[s]))
                        .fold(f32::MAX, f32::min);
                    let min_dist_b: f32 = selected
                        .iter()
                        .map(|&s| 1.0 - cosine_similarity(&embeddings[b], &embeddings[s]))
                        .fold(f32::MAX, f32::min);
                    min_dist_a
                        .partial_cmp(&min_dist_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            selected.push(best);
            remaining.retain(|&x| x != best);
        }

        // Compute diversity scores
        selected.sort();
        let mut diagnostics = Vec::new();

        for (i, ex) in ast.examples.iter().enumerate() {
            if !selected.contains(&i) {
                diagnostics.push(PassDiagnostic::RemovedExample {
                    reason: format!(
                        "Example '{}...' removed for diversity (kept {} of {})",
                        &ex.input[..ex.input.len().min(30)],
                        max,
                        n
                    ),
                });
            }
        }

        // Keep only selected, in original order
        let mut kept: Vec<ExampleNode> = Vec::new();
        for (i, ex) in ast.examples.drain(..).enumerate() {
            if selected.contains(&i) {
                kept.push(ex);
            }
        }

        // Compute diversity scores for kept examples
        if kept.len() > 1 {
            let kept_embeddings: Vec<Vec<f32>> =
                kept.iter().map(|e| embedder.embed(&e.input)).collect();
            let n_kept = kept.len();
            let scores: Vec<f64> = (0..n_kept)
                .map(|i| {
                    kept_embeddings
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, e)| (1.0 - cosine_similarity(&kept_embeddings[i], e)) as f64)
                        .sum::<f64>()
                        / (n_kept - 1).max(1) as f64
                })
                .collect();
            for (ex, score) in kept.iter_mut().zip(scores) {
                ex.diversity_score = score;
            }
        }

        ast.examples = kept;
        let changes_made = !diagnostics.is_empty();

        PassResult {
            ast,
            changes_made,
            diagnostics,
        }
    }
}
