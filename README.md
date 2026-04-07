# Prompt Compiler

A compiler for LLM prompts — lex, parse, optimize, and generate model-specific output.

Treats prompts like source code: eliminates redundancy, resolves contradictions, reorders for attention, and emits optimized formats for Claude, GPT, Mistral, and Llama.

## The #1 Rule: Never Make It Worse

The compiler includes a **safety net** that checks semantic similarity between the original and compiled prompts. If the compiled prompt drifts too far from the original intent, it can warn, fall back to the original, or abort:

```bash
# Warn on drift (default)
prompt-compiler prompt.txt --safety-action warn

# Fall back to original if drift detected
prompt-compiler prompt.txt --safety-action fallback --safety-threshold 0.85

# Abort with error on drift
prompt-compiler prompt.txt --safety-action abort
```

## Pipeline

```
Raw prompt string
       │
  ┌────▼────┐
  │  Lexer  │  regex rules → token stream
  └────┬────┘
  ┌────▼────┐
  │ Parser  │  tokens → PromptAST (with RawNode fallback for unclassifiable text)
  └────┬────┘
  ┌────▼──────────┐
  │   Optimizer   │  7 passes over the AST
  │               │
  │  1. DeadInstructionElimination   — remove semantically redundant instructions
  │  2. ContradictionResolver        — detect and resolve conflicting instructions
  │  3. AttentionAwareReorder        — critical content first (lost-in-middle fix)
  │  4. ContextRelevancePruning      — drop irrelevant context blocks
  │  5. ExampleDiversitySelection    — keep maximally diverse examples
  │  6. RedundancyElimination        — exact text deduplication
  │  7. NegativeToPositive           — "don't use jargon" → "use plain language"
  └────┬──────────┘
  ┌────▼──────────┐
  │  Safety Net   │  semantic similarity check — never make it worse
  └────┬──────────┘
  ┌────▼──────────┐
  │   Codegen     │  AST → model-specific format
  │               │
  │  Claude  → XML tags (<persona>, <instructions>, <examples>)
  │  GPT     → Markdown headers + bold critical rules
  │  Mistral → [INST] wrapper + compact bullets
  │  Llama   → special tokens + Step N: markers
  └────┬──────────┘
  ┌────▼──────────────┐
  │ Quality Estimator │  token count, clarity, structure, compatibility scores
  └───────────────────┘
```

## Installation

```bash
# Rust
cargo install --path .

# Python (via maturin — no Rust needed for end users)
pip install promptc
```

Or build from source:

```bash
cargo build --release
```

## Usage

### Basic compilation

```bash
# Compile a prompt for Claude (default)
prompt-compiler prompt.txt -t claude -O2

# Compile for GPT with output file
prompt-compiler prompt.txt -t gpt -o optimized.txt

# Compile for Mistral
prompt-compiler prompt.txt -t mistral

# Read from stdin
echo "## Instructions\n- Be concise.\n- Do not use jargon." | prompt-compiler - -t claude
```

### Optimization levels

```bash
-O0    # No optimization — just parse and codegen
-O1    # Safe passes only (context pruning, redundancy, negative-to-positive)
-O2    # All 7 passes (default)
```

### GPT-ism detection

Check a prompt for Claude-incompatible patterns without compiling:

```bash
prompt-compiler prompt.txt --check
```

Example output:

```
Found 3 GPT-ism(s):

  [Warning] 'let's think step by step'
    → Use <thinking> tags or 'Think through this step-by-step:' for Claude

  [Warning] 'As an AI language model'
    → Remove entirely — Claude doesn't need this preamble

  [Info] '**bold**'
    → Consider using <important>...</important> XML tags for Claude
```

### Safety net

The compiler checks that the compiled output doesn't drift semantically from the original:

```bash
# Warn on semantic drift (default)
prompt-compiler prompt.txt --safety-action warn --safety-threshold 0.85

# Fall back to original uncompiled prompt on drift
prompt-compiler prompt.txt --safety-action fallback

# Abort with error
prompt-compiler prompt.txt --safety-action abort
```

### Quality report

```bash
prompt-compiler prompt.txt -t claude --report
```

Prints optimized prompt to stdout and a quality report to stderr:

```
=== Quality Report ===
Tokens: 45 -> 38 (15.6% reduction)
Instruction clarity:    0.78
Structural improvement: 0.83
Model compatibility:    1.00
Overall delta:          0.61

Optimizer changes: 3
  - Removed: 'Always cite all sources.' (Redundant with 'You must cite all sources.')
  - Rewritten: 'Do not use jargon.' -> 'Use plain, accessible language.'
Compiled: 51 tokens (~, estimated ±5% — Anthropic tokenizer is not public)
```

Token counts are honest — approximate counts are clearly marked per target model.

### AST inspection

```bash
# Emit parsed AST as JSON (before optimization)
prompt-compiler prompt.txt --emit-ast

# Emit optimized AST as JSON
prompt-compiler prompt.txt --emit-optimized-ast
```

## Prompt format

The compiler recognizes section headers to structure the AST:

```
## Persona
You are a technical writer. You specialize in clear documentation.

## Instructions
- Always write in active voice.
- Do not use passive constructions.
- You must cite all sources.

## Constraints
- Keep responses under 500 words.

## Context
Active voice makes writing clearer and more direct.

## Examples
Input: The report was written by the team.
Output: The team wrote the report.

## Format
Respond in plain text paragraphs.
```

Headers can use `## Title`, `[TITLE]`, or bare keywords like `INSTRUCTIONS:`. If no headers are present, the compiler uses heuristic classification.

**Anything the parser can't classify goes into `RawNode` and passes through unchanged.** Silent failure is fine. Silent corruption is not.

## Library usage

### Rust

```rust
use prompt_compiler::{compile, ModelTarget};

let source = "## Instructions\n- Be concise.\n- Do not use jargon.";
let output = compile(source, ModelTarget::Claude, 2).unwrap();
println!("{output}");
```

With safety check:

```rust
use prompt_compiler::{compile_with_safety, ModelTarget, SafetyCheck, SafetyAction};

let safety = SafetyCheck::new(0.85, SafetyAction::Fallback);
let result = compile_with_safety(source, ModelTarget::Claude, 2, safety).unwrap();

if result.used_fallback {
    eprintln!("Safety net triggered — using original prompt");
}
println!("{}", result.text);
```

For fine-grained control:

```rust
use prompt_compiler::{lexer, parser, Optimizer, OptimizerOptions, ModelTarget, codegen};

let tokens = lexer::tokenize(source).unwrap();
let ast = parser::parse(tokens, source).unwrap();

let optimizer = Optimizer::new(ModelTarget::Claude, OptimizerOptions::default());
let result = optimizer.run(ast);

let gen = codegen::for_target(ModelTarget::Claude);
let output = gen.render(&result.ast);
```

### Python

```python
from promptc import compile, check_gptisms

# Compile a prompt for Claude
compiled = compile("## Instructions\n- Be concise.", target="claude", opt_level=2)
print(compiled)

# Check for GPT-isms
findings = check_gptisms("Let's think step by step.")
for found, suggestion, severity in findings:
    print(f"[{severity}] {found} -> {suggestion}")
```

## Embedding options

The default embedder uses **TF-IDF** (pure Rust, no dependencies, offline).

For higher accuracy, enable **fastembed-rs** (bundles all-MiniLM-L6-v2, 22MB ONNX model):

```bash
cargo build --features fastembed
```

## Benchmarks

Run the benchmark suite against 10 real-world prompts:

```bash
./benchmarks/run_bench.sh --target claude
```

Results are saved to `benchmarks/results/latest.json`.

## How it works

All analysis is **rule-based and local** — no LLM calls, no API keys, no network access:

- **Semantic similarity** uses TF-IDF vectors with cosine similarity (or fastembed-rs for neural embeddings)
- **Polarity detection** uses regex matching for negation words
- **Priority classification** uses keyword tier lookups
- **Negative-to-positive rewriting** uses a static regex → replacement table
- **GPT-ism detection** uses 10 hardcoded regex patterns
- **Safety net** compares embeddings of original vs compiled prompt

This makes it fast, deterministic, and offline.

## CI/CD

GitHub Actions runs on every push and PR:
- `cargo test --all` — all unit + integration tests
- `cargo clippy -- -D warnings` — lint
- `cargo fmt --check` — formatting

Release workflow builds Python wheels for all platforms via maturin.

## Tests

```bash
cargo test
```

72 tests: 54 unit tests covering every module + 18 integration tests for the full pipeline.

## License

MIT
