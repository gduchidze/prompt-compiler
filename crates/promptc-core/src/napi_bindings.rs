use napi_derive::napi;

use crate::analysis::{gptisms, quality};
use crate::codegen::{self, ModelTarget};
use crate::embedder::TfIdfEmbedder;
use crate::optimizer::pass::PassDiagnostic;
use crate::optimizer::{Optimizer, OptimizerOptions};
use crate::safety::{SafetyCheck, SafetyResult};
use crate::{lexer, parser};

#[napi(object)]
pub struct CompileResult {
    pub output: String,
    pub token_reduction_pct: f64,
    pub quality_delta: f64,
    pub changes: Vec<ChangeRecord>,
    pub warnings: Vec<String>,
    pub safety_similarity: f64,
}

#[napi(object)]
pub struct ChangeRecord {
    pub kind: String,
    pub description: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[napi(object)]
pub struct LintIssue {
    pub rule: String,
    pub severity: String,
    pub found: String,
    pub suggestion: String,
    pub start: u32,
    pub end: u32,
}

fn parse_target(target: &str) -> napi::Result<ModelTarget> {
    match target.to_lowercase().as_str() {
        "claude" => Ok(ModelTarget::Claude),
        "gpt" => Ok(ModelTarget::Gpt),
        "mistral" => Ok(ModelTarget::Mistral),
        "llama" => Ok(ModelTarget::Llama),
        _ => Err(napi::Error::from_reason(format!(
            "Unknown target '{target}'. Expected: claude, gpt, mistral, llama"
        ))),
    }
}

fn diagnostic_to_record(diag: &PassDiagnostic) -> ChangeRecord {
    match diag {
        PassDiagnostic::RemovedInstruction { text, reason } => ChangeRecord {
            kind: "removed_instruction".into(),
            description: reason.clone(),
            before: Some(text.clone()),
            after: None,
        },
        PassDiagnostic::ConvertedPolarity { before, after } => ChangeRecord {
            kind: "converted_polarity".into(),
            description: "Rewrote negative directive to positive".into(),
            before: Some(before.clone()),
            after: Some(after.clone()),
        },
        PassDiagnostic::PrunedContext { text, relevance } => ChangeRecord {
            kind: "pruned_context".into(),
            description: format!("Relevance score {:.2} below threshold", relevance),
            before: Some(text.clone()),
            after: None,
        },
        PassDiagnostic::ContradictionResolved { kept, removed } => ChangeRecord {
            kind: "contradiction_resolved".into(),
            description: format!("Kept '{}', removed contradicting instruction", kept),
            before: Some(removed.clone()),
            after: Some(kept.clone()),
        },
        PassDiagnostic::ContradictionFound { text_a, text_b } => ChangeRecord {
            kind: "contradiction_found".into(),
            description: format!("Contradiction between instructions"),
            before: Some(text_a.clone()),
            after: Some(text_b.clone()),
        },
        PassDiagnostic::ReorderedInstruction { from, to, reason } => ChangeRecord {
            kind: "reordered".into(),
            description: format!("{} (position {} -> {})", reason, from, to),
            before: None,
            after: None,
        },
        PassDiagnostic::MergedInstructions { kept, removed } => ChangeRecord {
            kind: "merged".into(),
            description: format!("Merged instruction {:?} into {:?}", removed, kept),
            before: None,
            after: None,
        },
        PassDiagnostic::RemovedExample { reason } => ChangeRecord {
            kind: "removed_example".into(),
            description: reason.clone(),
            before: None,
            after: None,
        },
    }
}

/// Compile a prompt source for a target model.
///
/// Returns a CompileResult with the optimized output, token reduction,
/// quality metrics, changes made, and safety similarity score.
#[napi]
pub fn compile(source: String, target: String, opt_level: u8) -> napi::Result<CompileResult> {
    let model_target = parse_target(&target)?;

    // Lex + Parse
    let tokens = lexer::tokenize(&source)
        .map_err(|e| napi::Error::from_reason(format!("Lex error: {e}")))?;
    let ast = parser::parse(tokens, &source)
        .map_err(|e| napi::Error::from_reason(format!("Parse error: {e}")))?;

    let before_ast = ast.clone();

    // Optimize
    let opts = OptimizerOptions {
        optimization_level: opt_level,
        ..Default::default()
    };
    let optimizer = Optimizer::new(model_target, opts);
    let opt_result = optimizer.run(ast);

    // Codegen
    let gen = codegen::for_target(model_target);
    let compiled = gen.render(&opt_result.ast);

    // Safety check
    let safety = SafetyCheck::default();
    let docs = [source.as_str(), compiled.as_str()];
    let embedder = TfIdfEmbedder::from_documents(&docs);
    let safety_result: SafetyResult = safety
        .check(&source, &compiled, &embedder)
        .map_err(|e| napi::Error::from_reason(format!("Safety error: {e}")))?;

    // Quality report
    let report = quality::compute_quality(
        &before_ast,
        &opt_result.ast,
        opt_result.diagnostics.clone(),
        &source,
    );

    // Map diagnostics to change records
    let changes: Vec<ChangeRecord> = opt_result
        .diagnostics
        .iter()
        .map(diagnostic_to_record)
        .collect();

    // Collect warnings
    let mut warnings = Vec::new();
    if let Some(w) = &safety_result.warning {
        warnings.push(w.clone());
    }

    Ok(CompileResult {
        output: compiled,
        token_reduction_pct: report.token_reduction_pct,
        quality_delta: report.overall_delta,
        changes,
        warnings,
        safety_similarity: safety_result.similarity,
    })
}

/// Parse a prompt source into an AST and return it as JSON.
#[napi]
pub fn parse(source: String) -> napi::Result<String> {
    let tokens = lexer::tokenize(&source)
        .map_err(|e| napi::Error::from_reason(format!("Lex error: {e}")))?;
    let ast = parser::parse(tokens, &source)
        .map_err(|e| napi::Error::from_reason(format!("Parse error: {e}")))?;
    serde_json::to_string_pretty(&ast)
        .map_err(|e| napi::Error::from_reason(format!("Serialization error: {e}")))
}

/// Lint a prompt source for GPT-isms and incompatibilities.
#[napi]
pub fn lint(source: String, target: String) -> napi::Result<Vec<LintIssue>> {
    let _model_target = parse_target(&target)?;

    let findings = gptisms::detect_gptisms(&source);

    Ok(findings
        .into_iter()
        .map(|f| LintIssue {
            rule: f.pattern.to_string(),
            severity: format!("{:?}", f.severity).to_lowercase(),
            found: f.found,
            suggestion: f.suggestion,
            start: f.span.start as u32,
            end: f.span.end as u32,
        })
        .collect())
}
