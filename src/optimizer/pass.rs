use crate::codegen::ModelTarget;
use crate::embedder::Embedder;
use crate::parser::ast::*;
use crate::token_counter::TokenCounter;

pub struct PassContext<'a> {
    pub target: ModelTarget,
    pub opt_level: u8,
    pub embedder: &'a dyn Embedder,
    pub token_counter: &'a dyn TokenCounter,
    pub similarity_threshold: f32,
    pub context_relevance_threshold: f64,
    pub max_examples: usize,
}

#[derive(Debug, Clone)]
pub enum PassDiagnostic {
    RemovedInstruction {
        text: String,
        reason: String,
    },
    ReorderedInstruction {
        from: usize,
        to: usize,
        reason: String,
    },
    MergedInstructions {
        kept: NodeId,
        removed: NodeId,
    },
    PrunedContext {
        text: String,
        relevance: f64,
    },
    ConvertedPolarity {
        before: String,
        after: String,
    },
    RemovedExample {
        reason: String,
    },
    ContradictionFound {
        text_a: String,
        text_b: String,
    },
    ContradictionResolved {
        kept: String,
        removed: String,
    },
}

pub struct PassResult {
    pub ast: PromptAst,
    pub changes_made: bool,
    pub diagnostics: Vec<PassDiagnostic>,
}

impl PassResult {
    pub fn noop(ast: PromptAst) -> Self {
        Self {
            ast,
            changes_made: false,
            diagnostics: Vec::new(),
        }
    }
}

pub trait OptimizerPass: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self, ast: PromptAst, ctx: &PassContext<'_>) -> PassResult;
}
