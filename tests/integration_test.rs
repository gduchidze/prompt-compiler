use prompt_compiler::{compile, ModelTarget, PromptAst};

const SAMPLE_SIMPLE: &str = "\
## Instructions
- Write clear summaries.
- Do not use jargon.
- You must always cite sources.
";

const SAMPLE_WITH_GPTISMS: &str = "\
As an AI language model, I will help you. Let's think step by step.
### Instructions
Use **bold** for important things.
";

const SAMPLE_FULL: &str = "\
## Persona
You are a technical writer. You specialize in documentation.

## Instructions
- Always write in active voice.
- Do not use passive constructions.
- You must cite all sources.
- You must cite all sources.

## Context
Active voice makes writing clearer and more direct.

## Examples
Input: The report was written by the team.
Output: The team wrote the report.

## Format
Respond in plain text paragraphs.
";

#[test]
fn test_full_pipeline_simple_claude() {
    let result = compile(SAMPLE_SIMPLE, ModelTarget::Claude, 2).unwrap();
    assert!(!result.is_empty());
    assert!(result.contains("<instructions>"));
}

#[test]
fn test_full_pipeline_simple_gpt() {
    let result = compile(SAMPLE_SIMPLE, ModelTarget::Gpt, 2).unwrap();
    assert!(!result.is_empty());
    assert!(result.contains("## Instructions"));
}

#[test]
fn test_full_pipeline_simple_mistral() {
    let result = compile(SAMPLE_SIMPLE, ModelTarget::Mistral, 2).unwrap();
    assert!(result.contains("[INST]"));
    assert!(result.contains("[/INST]"));
}

#[test]
fn test_full_pipeline_simple_llama() {
    let result = compile(SAMPLE_SIMPLE, ModelTarget::Llama, 2).unwrap();
    assert!(result.contains("<|system|>"));
    assert!(result.contains("Step 1:"));
}

#[test]
fn test_full_pipeline_gptisms_detected() {
    let findings = prompt_compiler::analysis::gptisms::detect_gptisms(SAMPLE_WITH_GPTISMS);
    assert!(
        findings.len() >= 2,
        "Expected at least 2 GPT-isms, found {}",
        findings.len()
    );
}

#[test]
fn test_full_pipeline_deduplication() {
    // Parse, then optimize — should remove the duplicate "cite all sources"
    let tokens = prompt_compiler::lexer::tokenize(SAMPLE_FULL).unwrap();
    let ast = prompt_compiler::parser::parse(tokens, SAMPLE_FULL).unwrap();
    let before_count = ast.instructions.len();

    let optimizer = prompt_compiler::Optimizer::new(
        ModelTarget::Claude,
        prompt_compiler::OptimizerOptions {
            optimization_level: 2,
            ..Default::default()
        },
    );
    let result = optimizer.run(ast);
    let after_count = result.ast.instructions.len();

    assert!(
        after_count < before_count,
        "Expected deduplication: before={before_count}, after={after_count}"
    );
}

#[test]
fn test_codegen_claude_has_xml() {
    let result = compile(SAMPLE_FULL, ModelTarget::Claude, 2).unwrap();
    assert!(result.contains("<persona>"), "Missing <persona> tag");
    assert!(result.contains("<instructions>"), "Missing <instructions> tag");
}

#[test]
fn test_codegen_gpt_has_markdown() {
    let result = compile(SAMPLE_FULL, ModelTarget::Gpt, 2).unwrap();
    assert!(result.contains("##"), "Missing markdown headers");
}

#[test]
fn test_codegen_mistral_has_inst() {
    let result = compile(SAMPLE_FULL, ModelTarget::Mistral, 2).unwrap();
    assert!(result.contains("[INST]"));
}

#[test]
fn test_quality_report() {
    let tokens = prompt_compiler::lexer::tokenize(SAMPLE_FULL).unwrap();
    let ast = prompt_compiler::parser::parse(tokens, SAMPLE_FULL).unwrap();
    let before = ast.clone();

    let optimizer = prompt_compiler::Optimizer::new(
        ModelTarget::Claude,
        prompt_compiler::OptimizerOptions {
            optimization_level: 2,
            ..Default::default()
        },
    );
    let result = optimizer.run(ast);

    let report = prompt_compiler::analysis::quality::compute_quality(
        &before,
        &result.ast,
        result.diagnostics,
        SAMPLE_FULL,
    );

    // Should have positive structural improvement (has persona, format, examples)
    assert!(
        report.structural_improvement > 0.0,
        "Expected positive structural improvement"
    );
    assert!(report.overall_delta >= 0.0);
}

#[test]
fn test_roundtrip_ast_json() {
    let tokens = prompt_compiler::lexer::tokenize(SAMPLE_FULL).unwrap();
    let ast = prompt_compiler::parser::parse(tokens, SAMPLE_FULL).unwrap();

    let json = serde_json::to_string(&ast).unwrap();
    let deserialized: PromptAst = serde_json::from_str(&json).unwrap();

    assert_eq!(ast.instructions.len(), deserialized.instructions.len());
    assert_eq!(ast.metadata.source_hash, deserialized.metadata.source_hash);
    assert_eq!(ast.examples.len(), deserialized.examples.len());
}

#[test]
fn test_o0_no_changes() {
    let result = compile(SAMPLE_FULL, ModelTarget::Claude, 0).unwrap();
    // O0 should still produce output (just no optimization)
    assert!(!result.is_empty());
}

#[test]
fn test_negative_to_positive_claude_only() {
    // Claude should rewrite negatives
    let claude_result = compile("## Instructions\n- Do not use jargon.\n", ModelTarget::Claude, 2).unwrap();
    // GPT should preserve negatives
    let gpt_result = compile("## Instructions\n- Do not use jargon.\n", ModelTarget::Gpt, 2).unwrap();

    // Claude should have rewritten the negative
    assert!(
        !claude_result.contains("Do not use jargon"),
        "Claude output should have rewritten the negative instruction"
    );
    // GPT should preserve it
    assert!(
        gpt_result.contains("Do not use jargon"),
        "GPT output should preserve the negative instruction"
    );
}
