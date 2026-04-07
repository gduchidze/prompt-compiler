pub mod pass;
pub mod passes;

use crate::codegen::ModelTarget;
use crate::embedder::TfIdfEmbedder;
use crate::parser::ast::PromptAst;
use crate::token_counter::WhitespaceCounter;
use pass::{OptimizerPass, PassContext, PassDiagnostic};
use passes::{
    attention_reorder::AttentionAwareReorder, contradiction::ContradictionResolver,
    context_prune::ContextRelevancePruning, dead_instruction::DeadInstructionElimination,
    example_diversity::ExampleDiversitySelection, negative_to_positive::NegativeToPositive,
    redundancy::RedundancyElimination,
};

pub struct OptimizerOptions {
    pub optimization_level: u8,
    pub similarity_threshold: f32,
    pub context_relevance_threshold: f64,
    pub max_examples: usize,
}

impl Default for OptimizerOptions {
    fn default() -> Self {
        Self {
            optimization_level: 2,
            similarity_threshold: 0.85,
            context_relevance_threshold: 0.1,
            max_examples: 5,
        }
    }
}

pub struct OptimizerOutput {
    pub ast: PromptAst,
    pub diagnostics: Vec<PassDiagnostic>,
}

pub struct Optimizer {
    target: ModelTarget,
    options: OptimizerOptions,
}

impl Optimizer {
    pub fn new(target: ModelTarget, options: OptimizerOptions) -> Self {
        Self { target, options }
    }

    pub fn run(&self, ast: PromptAst) -> OptimizerOutput {
        let all_passes: Vec<Box<dyn OptimizerPass>> = match self.options.optimization_level {
            0 => vec![],
            1 => vec![
                Box::new(ContextRelevancePruning),
                Box::new(RedundancyElimination),
                Box::new(NegativeToPositive),
            ],
            _ => vec![
                Box::new(DeadInstructionElimination),
                Box::new(ContradictionResolver),
                Box::new(AttentionAwareReorder),
                Box::new(ContextRelevancePruning),
                Box::new(ExampleDiversitySelection),
                Box::new(RedundancyElimination),
                Box::new(NegativeToPositive),
            ],
        };

        // Build embedder from all text in AST
        let all_texts = collect_texts(&ast);
        let text_refs: Vec<&str> = all_texts.iter().map(|s| s.as_str()).collect();
        let embedder = TfIdfEmbedder::from_documents(&text_refs);
        let counter = WhitespaceCounter;

        let ctx = PassContext {
            target: self.target,
            opt_level: self.options.optimization_level,
            embedder: &embedder,
            token_counter: &counter,
            similarity_threshold: self.options.similarity_threshold,
            context_relevance_threshold: self.options.context_relevance_threshold,
            max_examples: self.options.max_examples,
        };

        let mut current = ast;
        let mut all_diagnostics = Vec::new();

        for pass in &all_passes {
            let result = pass.run(current, &ctx);
            current = result.ast;
            all_diagnostics.extend(result.diagnostics);
        }

        OptimizerOutput {
            ast: current,
            diagnostics: all_diagnostics,
        }
    }
}

fn collect_texts(ast: &PromptAst) -> Vec<String> {
    let mut texts = Vec::new();
    if let Some(p) = &ast.persona {
        texts.push(p.text.clone());
    }
    for i in &ast.instructions {
        texts.push(i.text.clone());
    }
    for c in &ast.constraints {
        texts.push(c.text.clone());
    }
    for c in &ast.context {
        texts.push(c.text.clone());
    }
    for e in &ast.examples {
        texts.push(e.input.clone());
        texts.push(e.output.clone());
    }
    if let Some(f) = &ast.format_spec {
        texts.push(f.text.clone());
    }
    texts
}
