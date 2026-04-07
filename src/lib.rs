pub mod analysis;
pub mod codegen;
pub mod embedder;
pub mod error;
pub mod lexer;
pub mod optimizer;
pub mod parser;
pub mod token_counter;

pub use codegen::ModelTarget;
pub use error::CompilerError;
pub use optimizer::{Optimizer, OptimizerOptions};
pub use parser::ast::PromptAst;

/// High-level compile function for library users.
///
/// Lexes, parses, optimizes, and generates model-specific output in one call.
pub fn compile(source: &str, target: ModelTarget, opt_level: u8) -> Result<String, CompilerError> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens, source)?;
    let opts = OptimizerOptions {
        optimization_level: opt_level,
        ..Default::default()
    };
    let optimizer = Optimizer::new(target, opts);
    let result = optimizer.run(ast);
    let gen = codegen::for_target(target);
    Ok(gen.render(&result.ast))
}
