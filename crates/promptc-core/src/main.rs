use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use promptc_core::analysis::{gptisms, quality};
use promptc_core::codegen::{self, ModelTarget};
use promptc_core::optimizer::{Optimizer, OptimizerOptions};
use promptc_core::safety::{SafetyAction, SafetyCheck};
use promptc_core::token_counter;
use promptc_core::{lexer, parser};

#[derive(Parser)]
#[command(name = "prompt-compiler")]
#[command(about = "Compile LLM prompts — lex, parse, optimize, generate")]
#[command(version)]
struct Cli {
    /// Input file (use - for stdin)
    input: PathBuf,

    /// Target model
    #[arg(short = 't', long = "target", default_value = "claude")]
    #[arg(value_enum)]
    target: ModelTarget,

    /// Optimization level (0 = none, 1 = safe, 2 = full)
    #[arg(short = 'O', default_value = "2")]
    opt_level: u8,

    /// Output file (defaults to stdout)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Print quality report to stderr
    #[arg(long = "report")]
    report: bool,

    /// Emit AST as JSON (before optimization)
    #[arg(long = "emit-ast")]
    emit_ast: bool,

    /// Emit AST as JSON after optimization
    #[arg(long = "emit-optimized-ast")]
    emit_optimized_ast: bool,

    /// Run GPT-ism detector only (no compilation)
    #[arg(long = "check")]
    check_only: bool,

    /// Context relevance pruning threshold (0.0-1.0)
    #[arg(long = "context-threshold", default_value = "0.1")]
    context_threshold: f64,

    /// Maximum examples to retain
    #[arg(long = "max-examples", default_value = "5")]
    max_examples: usize,

    /// Safety net: minimum semantic similarity between original and compiled (0.0-1.0)
    #[arg(long = "safety-threshold", default_value = "0.85")]
    safety_threshold: f64,

    /// Safety net action on failure
    #[arg(long = "safety-action", default_value = "warn")]
    #[arg(value_enum)]
    safety_action: SafetyAction,
}

fn read_input(path: &PathBuf) -> Result<String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))
    }
}

fn write_output(content: &str, path: &Option<PathBuf>) -> Result<()> {
    match path {
        Some(p) => {
            std::fs::write(p, content)
                .with_context(|| format!("Failed to write to: {}", p.display()))?;
        }
        None => {
            print!("{content}");
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = read_input(&cli.input)?;

    // Check-only mode: run GPT-ism detector
    if cli.check_only {
        let findings = gptisms::detect_gptisms(&source);
        if findings.is_empty() {
            println!("No GPT-isms detected. Prompt looks clean!");
        } else {
            println!("Found {} GPT-ism(s):\n", findings.len());
            for finding in &findings {
                println!(
                    "  [{:?}] '{}'\n    → {}\n",
                    finding.severity, finding.found, finding.suggestion
                );
            }
        }
        return Ok(());
    }

    // Lex
    let tokens = lexer::tokenize(&source).context("Lexer failed")?;

    // Parse
    let ast = parser::parse(tokens, &source).context("Parser failed")?;

    // Emit raw AST
    if cli.emit_ast {
        let json = serde_json::to_string_pretty(&ast).context("Failed to serialize AST")?;
        write_output(&json, &cli.output)?;
        return Ok(());
    }

    // Clone before for quality report
    let before_ast = ast.clone();

    // Optimize
    let options = OptimizerOptions {
        optimization_level: cli.opt_level,
        similarity_threshold: 0.85,
        context_relevance_threshold: cli.context_threshold,
        max_examples: cli.max_examples,
    };
    let optimizer = Optimizer::new(cli.target, options);
    let opt_result = optimizer.run(ast);

    // Emit optimized AST
    if cli.emit_optimized_ast {
        let json =
            serde_json::to_string_pretty(&opt_result.ast).context("Failed to serialize AST")?;
        write_output(&json, &cli.output)?;
        return Ok(());
    }

    // Codegen
    let gen = codegen::for_target(cli.target);
    let compiled = gen.render(&opt_result.ast);

    // Safety check
    let safety = SafetyCheck::new(cli.safety_threshold, cli.safety_action);
    let docs = [source.as_str(), compiled.as_str()];
    let embedder = promptc_core::embedder::TfIdfEmbedder::from_documents(&docs);
    let safety_result = safety
        .check(&source, &compiled, &embedder)
        .context("Safety check failed")?;

    let output = if !safety_result.passed && cli.safety_action == SafetyAction::Fallback {
        eprintln!(
            "⚠ {}",
            safety_result
                .warning
                .as_deref()
                .unwrap_or("Semantic drift detected")
        );
        source.clone()
    } else {
        if let Some(warning) = &safety_result.warning {
            eprintln!("⚠ {warning}");
        }
        compiled
    };

    write_output(&output, &cli.output)?;

    // Quality report
    if cli.report {
        let report = quality::compute_quality(
            &before_ast,
            &opt_result.ast,
            opt_result.diagnostics,
            &source,
        );
        eprintln!("{report}");

        // Honest token count
        let tc = token_counter::count_tokens(&output, cli.target);
        eprintln!("Compiled: {tc}");
    }

    Ok(())
}
