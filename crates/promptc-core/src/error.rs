use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Lexer error at position {pos}: {message}")]
    LexError { pos: usize, message: String },

    #[error("Parser error: {message}")]
    ParseError { message: String },

    #[error("Optimizer pass '{pass}' failed: {reason}")]
    OptimizerError { pass: String, reason: String },

    #[error("Codegen error for target '{target}': {reason}")]
    CodegenError { target: String, reason: String },

    #[error("Semantic drift detected: similarity {similarity:.2} is below threshold {threshold:.2}")]
    SemanticDrift { similarity: f64, threshold: f64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
