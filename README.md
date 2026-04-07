# Prompt Compiler

A compiler for LLM prompts — lex, parse, optimize, and generate model-specific output.

Treats prompts like source code: eliminates redundancy, resolves contradictions, reorders for attention, and emits optimized formats for Claude, GPT, Mistral, and Llama.

## Pipeline

```
Raw prompt string
       │
  ┌────▼────┐
  │  Lexer  │  regex rules → token stream
  └────┬────┘
  ┌────▼────┐
  │ Parser  │  tokens → PromptAST (persona, instructions, context, examples, format)
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
  │   Codegen     │  AST → model-specific format
  │               │
  │  Claude  → XML tags (<persona>, <instructions>, <examples>)
  │  GPT     → Markdown headers + bold critical rules
  │  Mistral → [INST] wrapper + compact bullets
  │  Llama   → special tokens + Step N: markers
  └────┬──────────┘
  ┌────▼──────────────┐
  │ Quality Estimator │  token reduction, clarity, structure, compatibility scores
  └───────────────────┘
```

## Installation

```bash
cargo install --path .
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
  - Removed: 'Always cite all sources.' (Redundant with 'You must cite all sources.' (similarity=0.92))
  - Rewritten: 'Do not use jargon.' -> 'Use plain, accessible language.'
```

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

## Library usage

```rust
use prompt_compiler::{compile, ModelTarget};

let source = "## Instructions\n- Be concise.\n- Do not use jargon.";
let output = compile(source, ModelTarget::Claude, 2).unwrap();
println!("{output}");
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

## How it works

All analysis is **rule-based and local** — no LLM calls, no API keys, no network access:

- **Semantic similarity** uses TF-IDF vectors with cosine similarity
- **Polarity detection** uses regex matching for negation words
- **Priority classification** uses keyword tier lookups
- **Negative-to-positive rewriting** uses a static regex → replacement table
- **GPT-ism detection** uses 10 hardcoded regex patterns

This makes it fast, deterministic, and offline.

## Tests

```bash
cargo test
```

60 tests: 47 unit tests covering every module + 13 integration tests for the full pipeline.

## License

MIT
