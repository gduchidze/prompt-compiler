use std::fmt::Write;

use super::CodegenTarget;
use crate::parser::ast::*;

pub struct LlamaCodegen;

impl CodegenTarget for LlamaCodegen {
    fn name(&self) -> &'static str {
        "llama"
    }

    fn render(&self, ast: &PromptAst) -> String {
        let mut out = String::with_capacity(4096);

        writeln!(out, "<|system|>").unwrap();

        if let Some(p) = &ast.persona {
            writeln!(out, "You are {}.", p.role).unwrap();
            if !p.attributes.is_empty() {
                writeln!(out, "{}", p.attributes.join(" ")).unwrap();
            }
            writeln!(out).unwrap();
        }

        // Instructions with step markers
        for (i, inst) in ast.instructions.iter().enumerate() {
            writeln!(out, "Step {}: {}", i + 1, inst.text).unwrap();
        }

        if !ast.constraints.is_empty() {
            writeln!(out).unwrap();
            writeln!(out, "Rules:").unwrap();
            for c in &ast.constraints {
                writeln!(out, "- {}", c.text).unwrap();
            }
        }

        if !ast.context.is_empty() {
            writeln!(out).unwrap();
            writeln!(out, "Context:").unwrap();
            for ctx in &ast.context {
                writeln!(out, "{}", ctx.text).unwrap();
            }
        }

        if !ast.examples.is_empty() {
            writeln!(out).unwrap();
            writeln!(out, "Examples:").unwrap();
            for ex in &ast.examples {
                writeln!(out, "Input: {}", ex.input).unwrap();
                writeln!(out, "Output: {}", ex.output).unwrap();
            }
        }

        if let Some(fmt) = &ast.format_spec {
            writeln!(out).unwrap();
            writeln!(out, "Output format: {}", fmt.text).unwrap();
        }

        writeln!(out, "<|end|>").unwrap();
        writeln!(out, "<|user|>").unwrap();

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renders_special_tokens() {
        let ast = PromptAst::empty("test".into());
        let output = LlamaCodegen.render(&ast);
        assert!(output.contains("<|system|>"));
        assert!(output.contains("<|user|>"));
    }

    #[test]
    fn test_renders_step_markers() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Analyze the input".into(),
            verb: "analyze".into(),
            object: "the input".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });
        ast.instructions.push(InstructionNode {
            id: NodeId(1),
            text: "Provide a summary".into(),
            verb: "provide".into(),
            object: "a summary".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 3,
            embedding: None,
        });

        let output = LlamaCodegen.render(&ast);
        assert!(output.contains("Step 1: Analyze the input"));
        assert!(output.contains("Step 2: Provide a summary"));
    }
}
