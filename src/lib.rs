pub mod analysis;
pub mod codegen;
pub mod embedder;
pub mod error;
pub mod lexer;
pub mod optimizer;
pub mod parser;
#[cfg(feature = "python")]
pub mod python;
pub mod safety;
pub mod token_counter;

pub use codegen::ModelTarget;
pub use error::CompilerError;
pub use optimizer::{Optimizer, OptimizerOptions};
pub use parser::ast::PromptAst;
pub use safety::{SafetyAction, SafetyCheck, SafetyResult};

/// Result of compilation including safety information.
pub struct CompileOutput {
    pub text: String,
    pub safety: SafetyResult,
    pub used_fallback: bool,
}

/// High-level compile function for library users.
///
/// Lexes, parses, optimizes, generates model-specific output, and runs
/// the safety net to detect semantic drift.
pub fn compile(source: &str, target: ModelTarget, opt_level: u8) -> Result<String, CompilerError> {
    compile_with_safety(source, target, opt_level, SafetyCheck::default())
        .map(|out| out.text)
}

/// Compile with explicit safety check configuration.
pub fn compile_with_safety(
    source: &str,
    target: ModelTarget,
    opt_level: u8,
    safety: SafetyCheck,
) -> Result<CompileOutput, CompilerError> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens, source)?;
    let opts = OptimizerOptions {
        optimization_level: opt_level,
        ..Default::default()
    };
    let optimizer = Optimizer::new(target, opts);
    let result = optimizer.run(ast);
    let gen = codegen::for_target(target);
    let compiled = gen.render(&result.ast);

    // Build embedder from both texts for safety check
    let docs = [source, compiled.as_str()];
    let embedder = embedder::TfIdfEmbedder::from_documents(&docs);

    let safety_result = safety.check(source, &compiled, &embedder)?;

    if !safety_result.passed && safety.on_fail == SafetyAction::Fallback {
        return Ok(CompileOutput {
            text: source.to_string(),
            safety: safety_result,
            used_fallback: true,
        });
    }

    Ok(CompileOutput {
        text: compiled,
        safety: safety_result,
        used_fallback: false,
    })
}
