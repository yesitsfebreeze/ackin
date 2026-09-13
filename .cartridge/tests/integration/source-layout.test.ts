import { expect, test } from 'bun:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const runtime = path.resolve(import.meta.dir, '../../..');
function run(args: string[], cwd: string, timeout = 30_000) {
  const result = Bun.spawnSync(args, { cwd, stdout: 'pipe', stderr: 'pipe', timeout });
  expect(result.exitCode, `${args.join(' ')}\n${result.stderr.toString()}`).toBe(0);
  return result.stdout.toString().trim();
}

test('recorded memory source and SDK build from a recursive composition snapshot', () => {
  const registration = run(['git', 'worktree', 'list', '--porcelain'], runtime);
  const mainRuntime = registration.split('\n').find(line => line.startsWith('worktree '))!.slice(9);
  const source = path.dirname(mainRuntime), memorySource = path.join(source, 'memory.ctg');
  expect(fs.existsSync(path.join(source, '.gitmodules'))).toBe(true);
  const sourceRevision = run(['git', 'rev-parse', 'HEAD'], source);
  const memoryRegistrations = run(['git', 'worktree', 'list', '--porcelain'], memorySource);
  const memoryRevision = run(['git', 'rev-parse', 'HEAD'], memorySource);
  const temporary = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'cartridge-layout-')));
  try {
    const checkout = path.join(temporary, 'checkout');
    run(['git', 'clone', '--no-hardlinks', '--no-checkout', source, checkout], temporary);
    run(['git', 'checkout', '--detach', sourceRevision], checkout);
    const modulesFile = path.join(checkout, '.gitmodules');
    const modulesText = fs.readFileSync(modulesFile, 'utf8');
    const modules = run(['git', 'config', '-f', '.gitmodules', '--get-regexp', '^submodule\\..*\\.path$'], checkout).split('\n');
    for (const line of modules) {
      const boundary = line.indexOf(' '), key = line.slice(0, boundary), relative = line.slice(boundary + 1);
      expect(path.isAbsolute(relative)).toBe(false);
      expect(relative.split('/')).not.toContain('..');
      run(['git', 'config', key.slice(0, -4) + 'url', path.join(source, relative)], checkout);
    }
    run(['git', '-c', 'protocol.file.allow=always', 'submodule', 'update', '--init', '--recursive'], checkout, 120_000);
    expect(fs.readFileSync(modulesFile, 'utf8')).toBe(modulesText);
    const pinned = run(['git', 'ls-tree', 'HEAD', 'memory.ctg'], checkout).split(/\s+/)[2];
    const memory = path.join(checkout, 'memory.ctg'), clonedRuntime = path.join(checkout, 'cartridge.ctg');
    expect(run(['git', 'rev-parse', 'HEAD'], memory)).toBe(pinned);
    const metadata = JSON.parse(run(['cargo', 'metadata', '--no-deps', '--format-version=1', '--manifest-path', 'memory.ctg/Cargo.toml'], checkout));
    const pkg = metadata.packages.find((p: { name: string }) => p.name === 'memory');
    const sdk = pkg.dependencies.find((d: { name: string }) => d.name === 'cartridge').path;
    expect(fs.realpathSync(sdk)).toBe(fs.realpathSync(clonedRuntime));
    expect(fs.realpathSync(path.join(clonedRuntime, 'builtin/memory'))).toBe(memory);
    expect(fs.existsSync(path.join(memory, '.git'))).toBe(true);
    const proof: Record<string, unknown> = { root_commit: sourceRevision, memory_commit: pinned, submodules: modules.length, sdk_owner: path.relative(checkout, fs.realpathSync(sdk)), build: 'not requested' };
    if (process.env.CARTRIDGE_SOURCE_LAYOUT_BUILD === '1') {
      const start = performance.now();
      run(['just', 'build', 'memory', '--locked', '--features', 'cartridge', '--bin', 'memory_cartridge'], clonedRuntime, 600_000);
      const binary = path.join(clonedRuntime, 'target/debug/memory_cartridge');
      expect(fs.existsSync(binary)).toBe(true);
      proof.build = 'passed';
      proof.build_ms = performance.now() - start;
      proof.binary_sha256 = new Bun.CryptoHasher('sha256').update(fs.readFileSync(binary)).digest('hex');
    }
    expect(run(['git', 'status', '--porcelain', '--untracked-files=no'], memory)).toBe('');
    console.log('source-layout-evidence ' + JSON.stringify(proof));
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
  expect(run(['git', 'worktree', 'list', '--porcelain'], memorySource)).toBe(memoryRegistrations);
  expect(run(['git', 'rev-parse', 'HEAD'], memorySource)).toBe(memoryRevision);
  expect(run(['git', 'rev-parse', 'HEAD'], source)).toBe(sourceRevision);
}, 660_000);
