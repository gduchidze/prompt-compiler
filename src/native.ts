// napi-rs generated bindings — the .node file is produced by `npm run build:rs`
// eslint-disable-next-line @typescript-eslint/no-require-imports
const native = require('../promptc-core.node') as {
  compile(source: string, target: string, optLevel: number): {
    output: string;
    tokenReductionPct: number;
    qualityDelta: number;
    changes: { kind: string; description: string; before?: string; after?: string }[];
    warnings: string[];
    safetySimilarity: number;
  };
  parse(source: string): string;
  lint(source: string, target: string): {
    rule: string;
    severity: string;
    found: string;
    suggestion: string;
    start: number;
    end: number;
  }[];
};

export const { compile, parse, lint } = native;
