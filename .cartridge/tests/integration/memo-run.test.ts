import { expect, test } from 'bun:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
const runner = path.resolve(import.meta.dir, '../../tools/memo-run');
function fixture(body: string, args: string[] = [], namespace = '.cartridge/memos') {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'memo-run-')), directory = path.join(root, namespace, 'routine'), scratch = path.join(root, 'scratch');
  fs.mkdirSync(directory, { recursive: true }); fs.mkdirSync(scratch);
  const memo = path.join(directory, 'check.md'); fs.writeFileSync(memo, body);
  try { const result = Bun.spawnSync([runner, memo, ...args], { env: { ...process.env, TMPDIR: scratch }, stdout: 'pipe', stderr: 'pipe' }); expect(fs.readdirSync(scratch)).toEqual([]); return result; }
  finally { fs.rmSync(root, { recursive: true, force: true }); }
}
test('memo recipes preserve argument boundaries and propagate failure', () => {
  const body = '# Routine\n\n```just\nset positional-arguments\ncheck *args:\n    @printf "<%s>\\n" "$@"\nfail:\n    @exit 7\n```\n';
  const answer = fixture(body, ['check', 'two words', '$(touch never)', 'a;b']);
  expect(answer.exitCode).toBe(0); expect(answer.stdout.toString()).toBe('<two words>\n<$(touch never)>\n<a;b>\n');
  expect(fixture(body, ['fail']).exitCode).toBe(7);
  const nested = fixture('```just\ncheck:\n    @test "$PWD" = "$MEMO_OWNER_ROOT"\n```\n', ['check'], '.cartridge/development/memos');
  expect(nested.exitCode).toBe(0);
});
test('memo runner refuses missing, repeated and unterminated executable blocks', () => {
  for (const body of ['# No block\n', '```just\ncheck:\n    true\n', '```just\ncheck:\n    true\n```\n```just\nother:\n    true\n```']) expect(fixture(body).exitCode).toBe(2);
});
