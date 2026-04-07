use crate::analysis::gptisms::{detect_gptisms, GptismFinding};
use crate::optimizer::pass::PassDiagnostic;
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct QualityReport {
    pub before_tokens: usize,
    pub after_tokens: usize,
    pub token_reduction_pct: f64,
    pub instruction_clarity: f64,
    pub structural_improvement: f64,
    pub model_compatibility: f64,
    pub overall_delta: f64,
    pub diagnostics: Vec<PassDiagnostic>,
    pub gptism_findings: Vec<GptismFinding>,
}

pub fn compute_quality(
    before: &PromptAst,
    after: &PromptAst,
    diagnostics: Vec<PassDiagnostic>,
    original_text: &str,
) -> QualityReport {
    let before_tokens = before.metadata.total_tokens;
    let after_tokens = after.metadata.total_tokens;

    let token_reduction_pct = if before_tokens > 0 {
        (before_tokens as f64 - after_tokens as f64) / before_tokens as f64 * 100.0
    } else {
        0.0
    };

    let instruction_clarity = compute_instruction_clarity(after);
    let structural_improvement = compute_structural_improvement(after, &diagnostics);
    let gptism_findings = detect_gptisms(original_text);
    let model_compatibility = compute_model_compatibility(&gptism_findings);

    let token_reduction_normalized = (token_reduction_pct / 100.0).clamp(0.0, 1.0);
    let overall_delta = 0.35 * token_reduction_normalized
        + 0.25 * instruction_clarity
        + 0.25 * structural_improvement
        + 0.15 * model_compatibility;

    QualityReport {
        before_tokens,
        after_tokens,
        token_reduction_pct,
        instruction_clarity,
        structural_improvement,
        model_compatibility,
        overall_delta,
        diagnostics,
        gptism_findings,
    }
}

fn compute_instruction_clarity(ast: &PromptAst) -> f64 {
    if ast.instructions.is_empty() {
        return 0.5; // neutral
    }

    let total = ast.instructions.len() as f64;

    // % with non-empty verb
    let has_verb = ast.instructions.iter().filter(|i| !i.verb.is_empty()).count() as f64 / total;

    // % with Positive polarity
    let positive = ast
        .instructions
        .iter()
        .filter(|i| i.polarity == Polarity::Positive)
        .count() as f64
        / total;

    // % with explicit (non-Medium) priority
    let explicit_priority = ast
        .instructions
        .iter()
        .filter(|i| i.priority != Priority::Medium)
        .count() as f64
        / total;

    (has_verb + positive + explicit_priority) / 3.0
}

fn compute_structural_improvement(ast: &PromptAst, diagnostics: &[PassDiagnostic]) -> f64 {
    let mut score = 0.0;

    // Has persona
    if ast.persona.is_some() {
        score += 0.15;
    }

    // Has format spec
    if ast.format_spec.is_some() {
        score += 0.15;
    }

    // Has examples (up to 3 count)
    let ex_count = ast.examples.len().min(3) as f64;
    score += 0.1 * ex_count / 3.0;

    // Critical instructions at top
    if let Some(first) = ast.instructions.first() {
        if first.priority == Priority::Critical || first.priority == Priority::High {
            score += 0.2;
        }
    }

    // No contradictions
    let has_contradictions = diagnostics.iter().any(|d| {
        matches!(d, PassDiagnostic::ContradictionFound { .. })
    });
    if !has_contradictions {
        score += 0.2;
    }

    // Has instructions at all
    if !ast.instructions.is_empty() {
        score += 0.1;
    }

    score.min(1.0)
}

fn compute_model_compatibility(findings: &[GptismFinding]) -> f64 {
    (1.0 - findings.len() as f64 * 0.1).max(0.0)
}

impl std::fmt::Display for QualityReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Quality Report ===")?;
        writeln!(
            f,
            "Tokens: {} -> {} ({:.1}% reduction)",
            self.before_tokens, self.after_tokens, self.token_reduction_pct
        )?;
        writeln!(f, "Instruction clarity:    {:.2}", self.instruction_clarity)?;
        writeln!(
            f,
            "Structural improvement: {:.2}",
            self.structural_improvement
        )?;
        writeln!(f, "Model compatibility:    {:.2}", self.model_compatibility)?;
        writeln!(f, "Overall delta:          {:.2}", self.overall_delta)?;

        if !self.gptism_findings.is_empty() {
            writeln!(f, "\nGPT-ism findings:")?;
            for finding in &self.gptism_findings {
                writeln!(f, "  [{:?}] '{}' — {}", finding.severity, finding.found, finding.suggestion)?;
            }
        }

        let change_count = self.diagnostics.len();
        if change_count > 0 {
            writeln!(f, "\nOptimizer changes: {}", change_count)?;
            for diag in &self.diagnostics {
                match diag {
                    PassDiagnostic::RemovedInstruction { text, reason } => {
                        writeln!(f, "  - Removed: '{}' ({})", truncate(text, 50), reason)?;
                    }
                    PassDiagnostic::ConvertedPolarity { before, after } => {
                        writeln!(f, "  - Rewritten: '{}' -> '{}'", truncate(before, 40), truncate(after, 40))?;
                    }
                    PassDiagnostic::PrunedContext { text, relevance } => {
                        writeln!(f, "  - Pruned context: '{}' (relevance={:.2})", truncate(text, 40), relevance)?;
                    }
                    PassDiagnostic::ContradictionResolved { kept, removed } => {
                        writeln!(f, "  - Contradiction: kept '{}', removed '{}'", truncate(kept, 30), truncate(removed, 30))?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
