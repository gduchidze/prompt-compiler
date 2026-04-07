import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { PromptCompiler } from './compiler.js';
import type { Target, OptLevel, EvalTask } from './types.js';

const app = new Hono();
const compiler = new PromptCompiler();

app.post('/compile', async (c) => {
  const { source, target, optLevel } = await c.req.json<{
    source: string;
    target?: Target;
    optLevel?: OptLevel;
  }>();

  if (!source) {
    return c.json({ error: 'source is required' }, 400);
  }

  const result = compiler.compile(source, target ?? 'claude', optLevel ?? 2);
  return c.json(result);
});

app.post('/lint', async (c) => {
  const { source, target } = await c.req.json<{
    source: string;
    target?: Target;
  }>();

  if (!source) {
    return c.json({ error: 'source is required' }, 400);
  }

  const issues = compiler.lint(source, target ?? 'claude');
  return c.json(issues);
});

app.post('/parse', async (c) => {
  const { source } = await c.req.json<{ source: string }>();

  if (!source) {
    return c.json({ error: 'source is required' }, 400);
  }

  const ast = compiler.parse(source);
  return c.json(ast as Record<string, unknown>);
});

app.post('/eval', async (c) => {
  const { source, target, tasks } = await c.req.json<{
    source: string;
    target?: Target;
    tasks: EvalTask[];
  }>();

  if (!source || !tasks?.length) {
    return c.json({ error: 'source and tasks are required' }, 400);
  }

  const result = await compiler.compileAndEval(source, target ?? 'claude', tasks);
  return c.json(result);
});

app.get('/health', (c) => c.json({ status: 'ok' }));

export default app;

// Start server when run directly
const port = parseInt(process.env.PORT ?? '3000');
serve({ fetch: app.fetch, port }, () => {
  console.log(`promptc server running on http://localhost:${port}`);
});
