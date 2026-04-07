export type Target = 'claude' | 'gpt' | 'mistral' | 'llama';
export type OptLevel = 0 | 1 | 2;

export interface CompileResult {
  output: string;
  tokenReductionPct: number;
  qualityDelta: number;
  changes: ChangeRecord[];
  warnings: string[];
  safetySimilarity: number;
}

export interface ChangeRecord {
  kind: string;
  description: string;
  before?: string;
  after?: string;
}

export interface LintIssue {
  rule: string;
  severity: string;
  found: string;
  suggestion: string;
  start: number;
  end: number;
}

export interface EvalTask {
  name: string;
  input: string;
  rubric: string;
}

export interface EvalResult {
  task: string;
  original: number;
  compiled: number;
  delta: number;
}

export interface CompileAndEvalResult {
  compiled: CompileResult;
  originalScore: number;
  compiledScore: number;
  actualDelta: number;
  perTask: EvalResult[];
}

export interface AdaptiveResult {
  output: string;
  score: number;
  optLevel: OptLevel;
  iterations: number;
}
