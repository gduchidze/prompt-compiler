use std::fmt::Write;

use super::CodegenTarget;
use crate::parser::ast::*;

pub struct GptCodegen;

impl CodegenTarget for GptCodegen {
    fn name(&self) -> &'static str {
        "gpt"
    }

    fn render(&self, ast: &PromptAst) -> String {
        let mut out = String::with_capacity(4096);

        // System message section
        if let Some(p) = &ast.persona {
            writeln!(out, "## System").unwrap();
            writeln!(out, "You are {}.", p.role).unwrap();
            if !p.attributes.is_empty() {
                writeln!(out, "{}", p.attributes.join(" ")).unwrap();
            }
            writeln!(out).unwrap();
        }

        // Instructions
        if !ast.instructions.is_empty() {
            writeln!(out, "## Instructions").unwrap();
            for inst in &ast.instructions {
                if inst.priority == Priority::Critical {
                    writeln!(out, "**CRITICAL**: {}", inst.text).unwrap();
                } else if inst.priority == Priority::High {
                    writeln!(out, "**Important**: {}", inst.text).unwrap();
                } else {
                    writeln!(out, "- {}", inst.text).unwrap();
                }
            }
            writeln!(out).unwrap();
        }

        // Constraints
        if !ast.constraints.is_empty() {
            writeln!(out, "## Rules").unwrap();
            for c in &ast.constraints {
                if c.priority == Priority::Critical {
                    writeln!(out, "**MUST**: {}", c.text).unwrap();
                } else {
                    writeln!(out, "- {}", c.text).unwrap();
                }
            }
            writeln!(out).unwrap();
        }

        // Context
        if !ast.context.is_empty() {
            writeln!(out, "## Context").unwrap();
            for ctx in &ast.context {
                writeln!(out, "{}", ctx.text).unwrap();
            }
            writeln!(out).unwrap();
        }

        // Examples
        if !ast.examples.is_empty() {
            writeln!(out, "## Examples").unwrap();
            for (i, ex) in ast.examples.iter().enumerate() {
                writeln!(out, "**Example {}:**", i + 1).unwrap();
                writeln!(out, "- Input: {}", ex.input).unwrap();
                writeln!(out, "- Output: {}", ex.output).unwrap();
                writeln!(out).unwrap();
            }
        }

        // Format
        if let Some(fmt) = &ast.format_spec {
            writeln!(out, "## Output Format").unwrap();
            writeln!(out, "{}", fmt.text).unwrap();
        }

        // Raw nodes — pass through unchanged
        for raw in &ast.raw {
            writeln!(out).unwrap();
            writeln!(out, "{}", raw.text).unwrap();
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renders_markdown_headers() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Be helpful".into(),
            verb: "be".into(),
            object: "helpful".into(),
            polarity: Polarity::Positive,
            priority: Priority::Medium,
            span: TextSpan { start: 0, end: 0 },
            token_count: 2,
            embedding: None,
        });

        let output = GptCodegen.render(&ast);
        assert!(output.contains("## Instructions"));
    }

    #[test]
    fn test_renders_critical_as_bold() {
        let mut ast = PromptAst::empty("test".into());
        ast.instructions.push(InstructionNode {
            id: NodeId(0),
            text: "Never lie".into(),
            verb: "lie".into(),
            object: String::new(),
            polarity: Polarity::Negative,
            priority: Priority::Critical,
            span: TextSpan { start: 0, end: 0 },
            token_count: 2,
            embedding: None,
        });

        let output = GptCodegen.render(&ast);
        assert!(output.contains("**CRITICAL**"));
    }
}
