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

test('nested PRD lanes resolve their own memo owner', () => {
  const result = fixture('```just\ncheck:\n    @test "$MEMO_OWNER_ROOT/.cartridge/memos/routine/check.md" = "$MEMO_FILE"\n```\n', ['check'], '.cartridge/boards/runtime/.lanes/example/.cartridge/memos');
  expect(result.exitCode).toBe(0);
});

test('divergent lanes execute their own cold and warm builds without inherited caches', () => {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'lane-build-')));
  const repository = path.join(root, 'source'), shared = path.join(root, 'shared-target');
  const evidence: Record<string, unknown>[] = [];
  const run = (args: string[], cwd: string, env: Record<string, string | undefined> = process.env) => {
    const result = Bun.spawnSync(args, { cwd, env, stdout: 'pipe', stderr: 'pipe', timeout: 30_000 });
    expect(result.exitCode, result.stderr.toString()).toBe(0);
    return result.stdout.toString().trim();
  };
  const digest = (file: string) => new Bun.CryptoHasher('sha256').update(fs.readFileSync(file)).digest('hex');
  try {
    fs.mkdirSync(path.join(repository, 'src'), { recursive: true });
    fs.mkdirSync(path.join(repository, '.cartridge/memos/routine'), { recursive: true });
    fs.mkdirSync(path.join(repository, '.cartridge/tools'), { recursive: true });
    fs.writeFileSync(path.join(repository, 'Cargo.toml'), '[package]\nname = "cartridge"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n');
    fs.writeFileSync(path.join(repository, 'src/main.rs'), 'fn main() { println!("base"); }\n');
    fs.writeFileSync(path.join(repository, '.gitignore'), '/target/\n/Cargo.lock\n');
    fs.copyFileSync(runner, path.join(repository, '.cartridge/tools/memo-run'));
    fs.copyFileSync(path.resolve(import.meta.dir, '../../memos/routine/cartridge-development.md'), path.join(repository, '.cartridge/memos/routine/cartridge-development.md'));
    run(['git', 'init', '-b', 'main'], repository);
    run(['git', 'config', 'user.name', 'Lane Fixture'], repository);
    run(['git', 'config', 'user.email', 'lane@example.test'], repository);
    run(['git', 'add', '.'], repository);
    run(['git', '-c', 'commit.gpgsign=false', 'commit', '-m', 'fixture'], repository);
    const refusingWrapper = path.join(root, 'wrapper');
    fs.writeFileSync(refusingWrapper, '#!/bin/sh\necho "inherited wrapper was invoked" >&2\nexit 97\n', { mode: 0o755 });
    const toolchain = run(['rustc', '-vV'], repository);
    for (const name of ['alpha', 'bravo']) {
      const lane = path.join(root, '.cartridge/boards/runtime/.lanes', name);
      run(['git', 'worktree', 'add', '-b', name, lane], repository);
      fs.writeFileSync(path.join(lane, 'src/main.rs'), `fn main() { println!("${name}"); }\n`);
      run(['git', 'add', 'src/main.rs'], lane);
      run(['git', '-c', 'commit.gpgsign=false', 'commit', '-m', name], lane);
      const source = { commit: run(['git', 'rev-parse', 'HEAD'], lane), tree: run(['git', 'rev-parse', 'HEAD^{tree}'], lane), digest: digest(path.join(lane, 'src/main.rs')) };
      const modes = process.env.CARTRIDGE_COMPARE_WRAPPER ? ['safe', 'wrapper'] : ['safe'];
      for (const mode of modes) for (const temperature of ['cold', 'warm']) {
        const start = performance.now();
        const binary = path.join(lane, mode === 'safe' ? 'target' : 'wrapper-target', 'debug/cartridge');
        let output: string;
        if (mode === 'safe') {
          run([path.join(lane, '.cartridge/tools/memo-run'), path.join(lane, '.cartridge/memos/routine/cartridge-development.md'), 'build', 'runtime'], lane, {
            ...process.env, CARGO_TARGET_DIR: shared, RUSTC_WRAPPER: refusingWrapper, RUSTC_WORKSPACE_WRAPPER: refusingWrapper,
          });
          output = run([binary], lane);
          expect(output).toBe(name);
          expect(fs.existsSync(shared)).toBe(false);
        } else {
          const result = Bun.spawnSync(['cargo', 'build', '--offline'], { cwd: lane, env: {
            ...process.env, CARGO_TARGET_DIR: path.join(lane, 'wrapper-target'), RUSTC_WRAPPER: process.env.CARTRIDGE_COMPARE_WRAPPER!, RUSTC_WORKSPACE_WRAPPER: '',
          }, stdout: 'pipe', stderr: 'pipe', timeout: 30_000 });
          output = result.exitCode === 0 ? run([binary], lane) : `build failed (${result.exitCode}): ${result.stderr.toString()}`;
        }
        evidence.push({ lane: name, mode, temperature, ms: performance.now() - start, source, binary_sha256: fs.existsSync(binary) ? digest(binary) : null, expected: name, observed: output, correct: output === name, toolchain });
      }
    }
    console.log('lane-build-evidence ' + JSON.stringify(evidence));
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
}, 120_000);
