import { PromptCompiler } from './compiler.js';
import type { Target, OptLevel, EvalTask, AdaptiveResult } from './types.js';

/**
 * Adaptive compile loop: tries aggressive optimization first,
 * backs off if quality drops below threshold.
 */
export async function adaptiveCompile(
  source: string,
  target: Target,
  tasks: EvalTask[],
  opts?: { maxIterations?: number; apiKey?: string },
): Promise<AdaptiveResult> {
  const maxIterations = opts?.maxIterations ?? 3;
  const compiler = new PromptCompiler({ apiKey: opts?.apiKey });

  let optLevel: OptLevel = 2;
  let best: AdaptiveResult = { output: source, score: -1, optLevel, iterations: 0 };

  for (let i = 0; i < maxIterations; i++) {
    const compiled = compiler.compile(source, target, optLevel);
    const result = await compiler.compileAndEval(source, target, tasks);
    const avgScore = result.compiledScore;

    if (avgScore > best.score) {
      best = { output: compiled.output, score: avgScore, optLevel, iterations: i + 1 };
    }

    // Back off if quality dropped
    if (avgScore < best.score) {
      if (optLevel === 2) {
        optLevel = 1;
      } else if (optLevel === 1) {
        optLevel = 0;
      } else {
        break;
      }
    } else {
      break; // converged
    }
  }

  return best;
}
