import { compile, parse, lint } from './native.js';
import { GoogleGenerativeAI } from '@google/generative-ai';
import type {
  Target,
  OptLevel,
  CompileResult,
  LintIssue,
  EvalTask,
  CompileAndEvalResult,
} from './types.js';

function avg(nums: number[]): number {
  if (nums.length === 0) return 0;
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

function extractText(response: { candidates?: { content?: { parts?: { text?: string }[] } }[] }): string {
  return response.candidates?.[0]?.content?.parts?.[0]?.text ?? '';
}

function parseScore(text: string): number {
  const match = text.match(/([01](?:\.\d+)?)/);
  return match ? parseFloat(match[1]) : 0;
}

export class PromptCompiler {
  private genAI: GoogleGenerativeAI | null = null;

  constructor(opts?: { apiKey?: string }) {
    if (opts?.apiKey) {
      this.genAI = new GoogleGenerativeAI(opts.apiKey);
    } else if (process.env.GEMINI_API_KEY) {
      this.genAI = new GoogleGenerativeAI(process.env.GEMINI_API_KEY);
    }
  }

  /** Compile a prompt for a target model. Calls the Rust engine. */
  compile(source: string, target: Target = 'claude', optLevel: OptLevel = 2): CompileResult {
    return compile(source, target, optLevel);
  }

  /** Lint a prompt for GPT-isms and incompatibilities. */
  lint(source: string, target: Target = 'claude'): LintIssue[] {
    return lint(source, target);
  }

  /** Parse a prompt to AST (JSON). */
  parse(source: string): unknown {
    return JSON.parse(parse(source));
  }

  /** Compile and evaluate: run the same tasks against original and compiled prompts. */
  async compileAndEval(
    source: string,
    target: Target,
    tasks: EvalTask[],
  ): Promise<CompileAndEvalResult> {
    const compiled = this.compile(source, target, 2);

    const [originalScores, compiledScores] = await Promise.all([
      this.runEval(source, tasks),
      this.runEval(compiled.output, tasks),
    ]);

    return {
      compiled,
      originalScore: avg(originalScores),
      compiledScore: avg(compiledScores),
      actualDelta: avg(compiledScores) - avg(originalScores),
      perTask: tasks.map((t, i) => ({
        task: t.name,
        original: originalScores[i],
        compiled: compiledScores[i],
        delta: compiledScores[i] - originalScores[i],
      })),
    };
  }

  /** LLM-as-judge evaluation using Gemini. */
  private async runEval(prompt: string, tasks: EvalTask[]): Promise<number[]> {
    if (!this.genAI) {
      throw new Error(
        'Gemini API key required for evaluation. Pass apiKey in constructor or set GEMINI_API_KEY.',
      );
    }

    const model = this.genAI.getGenerativeModel({ model: 'gemini-2.0-flash' });

    return Promise.all(
      tasks.map(async (task) => {
        // Generate response using the prompt as system instruction
        const response = await model.generateContent({
          systemInstruction: prompt,
          contents: [{ role: 'user', parts: [{ text: task.input }] }],
        });

        const responseText = extractText(response.response);

        // Judge the response
        const judgment = await model.generateContent({
          systemInstruction: `Score the following response 0-1 on: ${task.rubric}\nRespond with ONLY a number between 0 and 1.`,
          contents: [{ role: 'user', parts: [{ text: responseText }] }],
        });

        return parseScore(extractText(judgment.response));
      }),
    );
  }
}
