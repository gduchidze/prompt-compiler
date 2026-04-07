pub mod claude;
pub mod gpt;
pub mod llama;
pub mod mistral;

use crate::parser::ast::PromptAst;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum ModelTarget {
    Claude,
    Gpt,
    Mistral,
    Llama,
}

impl std::fmt::Display for ModelTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelTarget::Claude => write!(f, "claude"),
            ModelTarget::Gpt => write!(f, "gpt"),
            ModelTarget::Mistral => write!(f, "mistral"),
            ModelTarget::Llama => write!(f, "llama"),
        }
    }
}

pub trait CodegenTarget {
    fn name(&self) -> &'static str;
    fn render(&self, ast: &PromptAst) -> String;
}

pub fn for_target(target: ModelTarget) -> Box<dyn CodegenTarget> {
    match target {
        ModelTarget::Claude => Box::new(claude::ClaudeCodegen),
        ModelTarget::Gpt => Box::new(gpt::GptCodegen),
        ModelTarget::Mistral => Box::new(mistral::MistralCodegen),
        ModelTarget::Llama => Box::new(llama::LlamaCodegen),
    }
}
