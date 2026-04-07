#!/usr/bin/env bash
# Benchmark runner for prompt-compiler
# Compiles all benchmark prompts and produces a summary report.
#
# Usage:
#   ./benchmarks/run_bench.sh [--target claude] [--opt-level 2]
#
# Requires: prompt-compiler binary in PATH or ./target/release/

set -euo pipefail

TARGET="${1:---target}"
TARGET_VAL="${2:-claude}"
OPT_LEVEL="${3:-2}"

# Find the binary
PROMPTC=""
if command -v prompt-compiler &>/dev/null; then
    PROMPTC="prompt-compiler"
elif [ -f "./target/release/prompt-compiler" ]; then
    PROMPTC="./target/release/prompt-compiler"
elif [ -f "./target/debug/prompt-compiler" ]; then
    PROMPTC="./target/debug/prompt-compiler"
else
    echo "Error: prompt-compiler binary not found. Run 'cargo build --release' first."
    exit 1
fi

PROMPTS_DIR="benchmarks/prompts"
RESULTS_DIR="benchmarks/results"
mkdir -p "$RESULTS_DIR"

echo "=== Prompt Compiler Benchmark ==="
echo "Target: $TARGET_VAL | Opt level: $OPT_LEVEL"
echo ""

TOTAL=0
IMPROVED=0
NEUTRAL=0
DEGRADED=0
TOTAL_BEFORE_TOKENS=0
TOTAL_AFTER_TOKENS=0

RESULTS_JSON="[]"

for prompt_file in "$PROMPTS_DIR"/*.txt; do
    filename=$(basename "$prompt_file")
    name="${filename%.txt}"
    TOTAL=$((TOTAL + 1))

    # Count tokens before (whitespace approximation)
    before_tokens=$(wc -w < "$prompt_file" | tr -d ' ')

    # Compile
    compiled=$($PROMPTC "$prompt_file" -t "$TARGET_VAL" -O "$OPT_LEVEL" 2>/dev/null || echo "COMPILE_ERROR")

    if [ "$compiled" = "COMPILE_ERROR" ]; then
        echo "  ✗ $name — compilation failed"
        DEGRADED=$((DEGRADED + 1))
        continue
    fi

    # Count tokens after
    after_tokens=$(echo "$compiled" | wc -w | tr -d ' ')

    # Calculate reduction
    if [ "$before_tokens" -gt 0 ]; then
        reduction=$(echo "scale=1; ($before_tokens - $after_tokens) * 100 / $before_tokens" | bc 2>/dev/null || echo "0")
    else
        reduction="0"
    fi

    TOTAL_BEFORE_TOKENS=$((TOTAL_BEFORE_TOKENS + before_tokens))
    TOTAL_AFTER_TOKENS=$((TOTAL_AFTER_TOKENS + after_tokens))

    # Check GPT-isms before and after
    gptisms_before=$($PROMPTC "$prompt_file" --check 2>/dev/null | head -1)
    gptisms_after=$(echo "$compiled" | $PROMPTC - --check 2>/dev/null | head -1)

    # Classify result
    if echo "$reduction" | grep -q "^-"; then
        DEGRADED=$((DEGRADED + 1))
        status="✗ degraded"
    elif [ "$after_tokens" -lt "$before_tokens" ]; then
        IMPROVED=$((IMPROVED + 1))
        status="✓ improved"
    else
        NEUTRAL=$((NEUTRAL + 1))
        status="○ neutral"
    fi

    printf "  %s %-35s %3s → %3s tokens (%s%%)\n" "$status" "$name" "$before_tokens" "$after_tokens" "$reduction"
done

echo ""
echo "=== Summary ==="
echo "Total prompts:     $TOTAL"
echo "Improved:          $IMPROVED/$TOTAL"
echo "Neutral:           $NEUTRAL/$TOTAL"
echo "Degraded:          $DEGRADED/$TOTAL"

if [ "$TOTAL_BEFORE_TOKENS" -gt 0 ]; then
    avg_reduction=$(echo "scale=1; ($TOTAL_BEFORE_TOKENS - $TOTAL_AFTER_TOKENS) * 100 / $TOTAL_BEFORE_TOKENS" | bc 2>/dev/null || echo "0")
    echo "Avg token reduction: ${avg_reduction}%"
fi

echo "Total tokens: $TOTAL_BEFORE_TOKENS → $TOTAL_AFTER_TOKENS"

# Save results
cat > "$RESULTS_DIR/latest.json" <<EOF
{
  "target": "$TARGET_VAL",
  "opt_level": $OPT_LEVEL,
  "total_prompts": $TOTAL,
  "improved": $IMPROVED,
  "neutral": $NEUTRAL,
  "degraded": $DEGRADED,
  "total_before_tokens": $TOTAL_BEFORE_TOKENS,
  "total_after_tokens": $TOTAL_AFTER_TOKENS
}
EOF

echo ""
echo "Results saved to $RESULTS_DIR/latest.json"
