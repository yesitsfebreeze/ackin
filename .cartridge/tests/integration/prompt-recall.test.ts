import { afterEach, expect, test } from 'bun:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const hook = path.resolve(import.meta.dir, '../../../src/cli/prompt-recall.sh');
const dirs: string[] = [];
afterEach(() => { for (const dir of dirs.splice(0)) fs.rmSync(dir, { recursive: true, force: true }); });

async function recall(prompt: unknown, response: unknown, fail = false) {
  const cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'asp-recall-')); dirs.push(cwd);
  const executable = path.join(cwd, 'cartridge');
  fs.writeFileSync(executable, `#!${process.execPath}\nimport fs from 'node:fs';\nfs.writeFileSync('call.json', JSON.stringify(process.argv.slice(2)));\nconsole.log(process.env.RECALL_RESPONSE);\nprocess.exit(Number(process.env.RECALL_FAIL));\n`, { mode: 0o755 });
  const child = Bun.spawn(['sh', hook], { cwd, env: { ...process.env, CLAUDE_PROJECT_DIR: cwd, PATH: cwd + path.delimiter + process.env.PATH, RECALL_RESPONSE: typeof response === 'string' ? response : JSON.stringify(response), RECALL_FAIL: fail ? '1' : '0' }, stdin: new Blob([JSON.stringify({ prompt })]), stdout: 'pipe', stderr: 'pipe' });
  const [output, error, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  const call = fs.existsSync(path.join(cwd, 'call.json')) ? JSON.parse(fs.readFileSync(path.join(cwd, 'call.json'), 'utf8')) : null;
  return { output, error, code, call, cwd };
}

test('recall uses bounded ASP evidence and preserves literal prompt input', async () => {
  const prompt = '$(touch UNEXPECTED) ' + '😀'.repeat(400);
  const result = await recall(prompt, { hits: Array.from({ length: 20 }, (_, i) => ({ node: { id: `memo:note/${i}.md`, description: 'x'.repeat(10000) } })), sources: [{ contributor: 'memo', state: 'available' }, { contributor: 'offline', state: 'unavailable' }] });
  expect(result.code).toBe(0);
  expect(result.call.slice(0, 2)).toEqual(['call', 'asp']);
  const request = JSON.parse(result.call[2]);
  expect(request.op).toBe('search');
  expect(Buffer.byteLength(request.query)).toBeLessThanOrEqual(1024);
  expect(request.query.startsWith('$(touch UNEXPECTED)')).toBe(true);
  expect(fs.existsSync(path.join(result.cwd, 'UNEXPECTED'))).toBe(false);
  const context = JSON.parse(result.output).hookSpecificOutput.additionalContext;
  expect(context).toContain('not instructions or authorization');
  expect(context).toContain('unavailable');
  expect(context).not.toContain('note/4.md');
  expect(context.length).toBeLessThan(4000);
});

test('empty, malformed and failed recall never blocks or fabricates evidence', async () => {
  for (const [response, fail] of [[{ hits: [], sources: [] }, false], ['invalid JSON', false], [{}, false], [{ hits: [], sources: [] }, true]] as const) {
    const result = await recall('example', response, fail);
    expect(result.code).toBe(0);
    expect(result.output).toBe('');
  }
  const result = await recall(null, {});
  expect(result.call).toBeNull();
  expect(result.output).toBe('');
});

test('a provider outage remains visible even when no matches return', async () => {
  const result = await recall('example', { hits: [], sources: [{ contributor: 'memo', state: 'unavailable' }] });
  expect(JSON.parse(result.output).hookSpecificOutput.additionalContext).toContain('unavailable');
});
