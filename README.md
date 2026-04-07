# promptc

A compiler for LLM prompts — lex, parse, optimize, and generate model-specific output.

Treats prompts like source code: eliminates redundancy, resolves contradictions, reorders for attention, and emits optimized formats for Claude, GPT, Mistral, and Llama.

## Architecture

```
[TypeScript / Hono]          <- API + CLI + LLM eval + reports
         |
[promptc-core (Rust/napi-rs)] <- parser + AST + 7 optimizer passes + codegen
         |
[LLM APIs (Gemini)]          <- eval + adaptive feedback loop
```

The Rust core handles all compute-heavy work (lexing, parsing, optimization, codegen, safety checks). The TypeScript layer provides the CLI, HTTP API, LLM-as-judge evaluation, and npm distribution.

## The #1 Rule: Never Make It Worse

The compiler includes a **safety net** that checks semantic similarity between the original and compiled prompts. If the compiled prompt drifts too far from the original intent, it can warn, fall back to the original, or abort.

## Pipeline

```
Raw prompt string
       |
  +----------+
  |  Lexer   |  regex rules -> token stream
  +----+-----+
  +----v-----+
  | Parser   |  tokens -> PromptAST (with RawNode fallback)
  +----+-----+
  +----v-----------+
  |   Optimizer    |  7 passes over the AST
  |                |
  |  1. DeadInstructionElimination   - remove semantically redundant instructions
  |  2. ContradictionResolver        - detect and resolve conflicting instructions
  |  3. AttentionAwareReorder        - critical content first (lost-in-middle fix)
  |  4. ContextRelevancePruning      - drop irrelevant context blocks
  |  5. ExampleDiversitySelection    - keep maximally diverse examples
  |  6. RedundancyElimination        - exact text deduplication
  |  7. NegativeToPositive           - "don't use jargon" -> "use plain language"
  +----+-----------+
  +----v-----------+
  |  Safety Net    |  semantic similarity check - never make it worse
  +----+-----------+
  +----v-----------+
  |   Codegen      |  AST -> model-specific format
  |                |
  |  Claude  -> XML tags (<persona>, <instructions>, <examples>)
  |  GPT     -> Markdown headers + bold critical rules
  |  Mistral -> [INST] wrapper + compact bullets
  |  Llama   -> special tokens + Step N: markers
  +----+-----------+
  +----v-----------------+
  | Quality Estimator    |  token count, clarity, structure, compatibility scores
  +----------------------+
```

## Installation

```bash
# npm (includes prebuilt native binaries)
npm install promptc

# CLI
npx promptc compile prompt.txt --target claude

# Rust (core only)
cargo install --path crates/promptc-core --features cli
```

Build from source:

```bash
# Build everything (Rust + TypeScript)
npm run build

# Rust only
cargo build --workspace --release
```

## Usage

### CLI

```bash
# Compile a prompt for Claude (default)
npx promptc compile prompt.txt -t claude -O 2

# Compile for GPT with output file
npx promptc compile prompt.txt -t gpt -o optimized.txt

# Full JSON output with metrics
npx promptc compile prompt.txt --json

# Lint for GPT-isms
npx promptc lint prompt.txt -t claude

# Parse to AST (JSON)
npx promptc parse prompt.txt

# Port from one model format to another
npx promptc port prompt.txt --from gpt --to claude

# Evaluate with LLM-as-judge (requires GEMINI_API_KEY)
npx promptc eval prompt.txt -t claude --suite eval-tasks.json
```

### Optimization levels

```bash
-O 0    # No optimization - just parse and codegen
-O 1    # Safe passes only (context pruning, redundancy, negative-to-positive)
-O 2    # All 7 passes (default)
```

### Legacy Rust CLI

```bash
prompt-compiler prompt.txt -t claude -O2
prompt-compiler prompt.txt --check          # GPT-ism detection only
prompt-compiler prompt.txt --report         # Quality report to stderr
prompt-compiler prompt.txt --emit-ast       # AST as JSON
prompt-compiler prompt.txt --safety-action fallback --safety-threshold 0.85
```

### API Server

```bash
npx ts-node src/server.ts
# or after build:
node dist/server.js
```

Endpoints:

```
POST /compile  { source, target?, optLevel? }  -> CompileResult
POST /lint     { source, target? }             -> LintIssue[]
POST /parse    { source }                      -> AST JSON
POST /eval     { source, target?, tasks }      -> CompileAndEvalResult
GET  /health                                   -> { status: "ok" }
```

## Library Usage

### TypeScript / Node.js

```typescript
import { PromptCompiler } from 'promptc';

const compiler = new PromptCompiler();

// Compile
const result = compiler.compile(source, 'claude', 2);
console.log(result.output);
console.log(`${result.tokenReductionPct}% token reduction`);
console.log(`Quality delta: ${result.qualityDelta}`);

// Lint
const issues = compiler.lint(source, 'claude');
for (const issue of issues) {
  console.log(`${issue.severity}: ${issue.found} -> ${issue.suggestion}`);
}

// Parse to AST
const ast = compiler.parse(source);

// Compile + evaluate with Gemini
const compiler = new PromptCompiler({ apiKey: process.env.GEMINI_API_KEY });
const evalResult = await compiler.compileAndEval(source, 'claude', [
  { name: 'clarity', input: 'Explain quantum computing', rubric: 'Clear and concise explanation' },
]);
console.log(`Original: ${evalResult.originalScore}, Compiled: ${evalResult.compiledScore}`);
```

### Adaptive compilation

```typescript
import { adaptiveCompile } from 'promptc';

// Tries aggressive optimization first, backs off if quality drops
const result = await adaptiveCompile(source, 'claude', tasks, {
  maxIterations: 3,
  apiKey: process.env.GEMINI_API_KEY,
});
console.log(`Best output at opt level ${result.optLevel}, score: ${result.score}`);
```

### Rust

```rust
use promptc_core::{compile, ModelTarget};

let source = "## Instructions\n- Be concise.\n- Do not use jargon.";
let output = compile(source, ModelTarget::Claude, 2).unwrap();
println!("{output}");
```

With safety check:

```rust
use promptc_core::{compile_with_safety, ModelTarget, SafetyCheck, SafetyAction};

let safety = SafetyCheck::new(0.85, SafetyAction::Fallback);
let result = compile_with_safety(source, ModelTarget::Claude, 2, safety).unwrap();

if result.used_fallback {
    eprintln!("Safety net triggered - using original prompt");
}
println!("{}", result.text);
```

Fine-grained control:

```rust
use promptc_core::{lexer, parser, Optimizer, OptimizerOptions, ModelTarget, codegen};

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

compiled = compile("## Instructions\n- Be concise.", target="claude", opt_level=2)
print(compiled)

findings = check_gptisms("Let's think step by step.")
for found, suggestion, severity in findings:
    print(f"[{severity}] {found} -> {suggestion}")
```

## Prompt Format

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

## Project Structure

```
promptc/
  Cargo.toml                    # Workspace manifest
  package.json                  # npm package config
  tsconfig.json
  crates/
    promptc-core/               # Rust engine
      src/
        lib.rs                  # Public API + napi bindings
        lexer/                  # Tokenization
        parser/                 # AST construction
        optimizer/              # 7-pass optimization pipeline
        codegen/                # Claude, GPT, Mistral, Llama output
        embedder/               # TF-IDF + optional fastembed
        analysis/               # GPT-ism detection + quality metrics
        safety.rs               # Semantic drift detection
      tests/
        integration_test.rs
  src/                          # TypeScript
    index.ts                    # Public exports
    compiler.ts                 # PromptCompiler class
    cli.ts                      # Commander CLI
    server.ts                   # Hono API server
    eval.ts                     # LLM-as-judge + adaptive loop
    report.ts                   # HTML report generation
    types.ts                    # TypeScript interfaces
    native.ts                   # napi-rs bridge
  benchmarks/
    prompts/                    # 10 real-world prompts
    tasks/                      # Eval task definitions
```

## Embedding Options

The default embedder uses **TF-IDF** (pure Rust, no dependencies, offline).

For higher accuracy, enable **fastembed-rs** (bundles all-MiniLM-L6-v2, 22MB ONNX model):

```bash
cargo build --features fastembed
```

## How It Works

All analysis is **rule-based and local** — no LLM calls required for compilation:

- **Semantic similarity** uses TF-IDF vectors with cosine similarity (or fastembed-rs for neural embeddings)
- **Polarity detection** uses regex matching for negation words
- **Priority classification** uses keyword tier lookups
- **Negative-to-positive rewriting** uses a static regex-to-replacement table
- **GPT-ism detection** uses 10 hardcoded regex patterns
- **Safety net** compares embeddings of original vs compiled prompt

The optional eval loop uses **Gemini** as an LLM-as-judge for quality scoring.

## CI/CD

GitHub Actions runs on every push and PR:
- `cargo test --workspace` — all unit + integration tests
- `cargo clippy --workspace -- -D warnings` — lint
- `cargo fmt --all --check` — formatting
- Node.js build verification

Release workflow cross-compiles native binaries for 6 platforms and publishes to npm.

## Tests

```bash
# Rust tests
cargo test --workspace

# TypeScript tests
npm test
```

72 Rust tests: 54 unit tests covering every module + 18 integration tests for the full pipeline.

## License

MIT
