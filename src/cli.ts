#!/usr/bin/env node

import { Command } from 'commander';
import { readFileSync, writeFileSync } from 'node:fs';
import { PromptCompiler } from './compiler.js';
import type { Target, OptLevel } from './types.js';

const compiler = new PromptCompiler();

function readInput(file: string): string {
  if (file === '-') {
    return readFileSync(0, 'utf-8');
  }
  return readFileSync(file, 'utf-8');
}

const program = new Command()
  .name('promptc')
  .description('Prompt compiler — parse, optimize, generate model-specific LLM prompts')
  .version('0.3.0');

program
  .command('compile')
  .description('Compile and optimize a prompt for a target model')
  .argument('<file>', 'Input file (use - for stdin)')
  .option('-t, --target <target>', 'Target model (claude, gpt, mistral, llama)', 'claude')
  .option('-O, --opt-level <level>', 'Optimization level (0, 1, 2)', '2')
  .option('-o, --output <file>', 'Output file (defaults to stdout)')
  .option('--json', 'Output full CompileResult as JSON')
  .action((file: string, opts: { target: string; optLevel: string; output?: string; json?: boolean }) => {
    const source = readInput(file);
    const result = compiler.compile(
      source,
      opts.target as Target,
      parseInt(opts.optLevel) as OptLevel,
    );

    if (opts.json) {
      const output = JSON.stringify(result, null, 2);
      if (opts.output) {
        writeFileSync(opts.output, output);
      } else {
        process.stdout.write(output + '\n');
      }
      return;
    }

    // Human-readable output
    console.error(`Token reduction: ${result.tokenReductionPct.toFixed(1)}%`);
    console.error(`Quality delta:   ${result.qualityDelta >= 0 ? '+' : ''}${result.qualityDelta.toFixed(3)}`);
    console.error(`Safety:          ${result.safetySimilarity.toFixed(3)}`);

    if (result.changes.length > 0) {
      console.error(`\nChanges (${result.changes.length}):`);
      for (const change of result.changes) {
        console.error(`  ${change.kind}: ${change.description}`);
      }
    }

    for (const warning of result.warnings) {
      console.error(`\nWarning: ${warning}`);
    }

    if (opts.output) {
      writeFileSync(opts.output, result.output);
    } else {
      process.stdout.write(result.output);
    }
  });

program
  .command('lint')
  .description('Lint a prompt for GPT-isms and incompatibilities')
  .argument('<file>', 'Input file (use - for stdin)')
  .option('-t, --target <target>', 'Target model', 'claude')
  .action((file: string, opts: { target: string }) => {
    const source = readInput(file);
    const issues = compiler.lint(source, opts.target as Target);

    if (issues.length === 0) {
      console.log('No issues found. Prompt looks clean!');
      return;
    }

    console.log(`Found ${issues.length} issue(s):\n`);
    for (const issue of issues) {
      const icon = issue.severity === 'warning' ? '\u2717' : '\u26A0';
      console.log(`  ${icon} ${issue.rule}`);
      console.log(`    Found: "${issue.found}"`);
      console.log(`    Suggestion: ${issue.suggestion}\n`);
    }

    process.exit(issues.some(i => i.severity === 'warning') ? 1 : 0);
  });

program
  .command('parse')
  .description('Parse a prompt to AST (JSON)')
  .argument('<file>', 'Input file (use - for stdin)')
  .option('-o, --output <file>', 'Output file (defaults to stdout)')
  .action((file: string, opts: { output?: string }) => {
    const source = readInput(file);
    const ast = compiler.parse(source);
    const json = JSON.stringify(ast, null, 2);

    if (opts.output) {
      writeFileSync(opts.output, json);
    } else {
      process.stdout.write(json + '\n');
    }
  });

program
  .command('eval')
  .description('Compile and evaluate with LLM-as-judge (requires GEMINI_API_KEY)')
  .argument('<file>', 'Input file')
  .option('-t, --target <target>', 'Target model', 'claude')
  .option('-s, --suite <file>', 'Eval suite JSON file')
  .action(async (file: string, opts: { target: string; suite?: string }) => {
    if (!opts.suite) {
      console.error('Error: --suite <file> is required for eval');
      process.exit(1);
    }

    const source = readInput(file);
    const tasks = JSON.parse(readFileSync(opts.suite, 'utf-8'));
    const result = await compiler.compileAndEval(source, opts.target as Target, tasks);

    console.log(`Original score:  ${result.originalScore.toFixed(3)}`);
    console.log(`Compiled score:  ${result.compiledScore.toFixed(3)}`);
    console.log(`Actual delta:    ${result.actualDelta >= 0 ? '+' : ''}${result.actualDelta.toFixed(3)}`);
    console.log(`Token reduction: ${result.compiled.tokenReductionPct.toFixed(1)}%`);

    console.log('\nPer-task breakdown:');
    for (const t of result.perTask) {
      console.log(`  ${t.task}: ${t.original.toFixed(3)} -> ${t.compiled.toFixed(3)} (${t.delta >= 0 ? '+' : ''}${t.delta.toFixed(3)})`);
    }
  });

program
  .command('port')
  .description('Port a prompt from one model format to another')
  .argument('<file>', 'Input file')
  .requiredOption('--from <target>', 'Source model format')
  .requiredOption('--to <target>', 'Destination model format')
  .option('-o, --output <file>', 'Output file (defaults to stdout)')
  .action((file: string, opts: { from: string; to: string; output?: string }) => {
    const source = readInput(file);
    // Parse with source target context, then re-emit for destination target
    const compiled = compiler.compile(source, opts.to as Target, 1);

    if (opts.output) {
      writeFileSync(opts.output, compiled.output);
    } else {
      process.stdout.write(compiled.output);
    }

    console.error(`Ported from ${opts.from} to ${opts.to} format`);
  });

program.parse();
