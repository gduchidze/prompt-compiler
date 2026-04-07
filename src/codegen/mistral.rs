use std::fmt::Write;

use super::CodegenTarget;
use crate::parser::ast::*;

pub struct MistralCodegen;

impl CodegenTarget for MistralCodegen {
    fn name(&self) -> &'static str {
        "mistral"
    }

    fn render(&self, ast: &PromptAst) -> String {
        let mut out = String::with_capacity(4096);

        writeln!(out, "[INST]").unwrap();

        if let Some(p) = &ast.persona {
            writeln!(out, "You are {}.", p.role).unwrap();
            if !p.attributes.is_empty() {
                writeln!(out, "{}", p.attributes.join(" ")).unwrap();
            }
            writeln!(out).unwrap();
        }

        for inst in &ast.instructions {
            writeln!(out, "• {}", inst.text).unwrap();
        }

        for c in &ast.constraints {
            writeln!(out, "• {}", c.text).unwrap();
        }

        if !ast.context.is_empty() {
            writeln!(out).unwrap();
            for ctx in &ast.context {
                writeln!(out, "{}", ctx.text).unwrap();
            }
        }

        if !ast.examples.is_empty() {
            writeln!(out).unwrap();
            for ex in &ast.examples {
                writeln!(out, "Input: {}", ex.input).unwrap();
                writeln!(out, "Output: {}", ex.output).unwrap();
            }
        }

        if let Some(fmt) = &ast.format_spec {
            writeln!(out).unwrap();
            writeln!(out, "{}", fmt.text).unwrap();
        }

        writeln!(out, "[/INST]").unwrap();

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renders_inst_tags() {
        let ast = PromptAst::empty("test".into());
        let output = MistralCodegen.render(&ast);
        assert!(output.contains("[INST]"));
        assert!(output.contains("[/INST]"));
    }

    #[test]
    fn test_renders_bullet_points() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Be concise".into(),
            verb: "be".into(),
            object: "concise".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 2,
            embedding: None,
        });

        let output = MistralCodegen.render(&ast);
        assert!(output.contains("• Be concise"));
    }
}
