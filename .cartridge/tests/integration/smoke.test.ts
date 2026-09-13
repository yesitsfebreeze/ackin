import { test, expect } from "bun:test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import net from "node:net";
import { randomBytes } from "node:crypto";

const runtime = path.resolve(import.meta.dir, "../../..");
const binary = path.join(process.env.CARGO_TARGET_DIR || path.join(runtime, "target"), "debug/cartridge");
const selected = process.env.CARTRIDGE_SMOKE || "all";
if (!["all", "policy", "proxy", "mcp"].includes(selected)) throw Error("Choose all, policy, proxy, or mcp");

function profile(root: string, name: string, port = 0) {
  const directory = path.join(root, name), builtin = path.join(directory, "builtin");
  fs.mkdirSync(builtin, { recursive: true });
  const init = name === "policy" ? 'return {{id="policy",path="policy"}}\n'
    : fs.readFileSync(path.join(runtime, ".cartridge", name, "init.lua"), "utf8");
  fs.writeFileSync(path.join(directory, "init.lua"), init);
  for (const match of init.matchAll(/path\s*=\s*"([^"\n]+)"/g)) {
    const module = match[1], destination = path.join(builtin, module);
    if (name === "proxy" && module === "router") {
      fs.mkdirSync(destination);
      fs.writeFileSync(path.join(destination, "cartridge.json"), JSON.stringify({ name: module, entry: "init.lua" }));
      fs.writeFileSync(path.join(destination, "init.lua"), 'return {provide={"router"},apply=function(ctx) ctx:provide("router",function() return {data={}} end) end}');
    } else fs.symlinkSync(fs.realpathSync(path.join(runtime, "builtin", module)), destination);
  }
  fs.writeFileSync(path.join(directory, "config.lua"), `return {
    sessions={dir="sessions"},memory={dir="memory",reason={url=""},tick={interval_secs=0},queue={enabled=false}},
    router={listen={"127.0.0.1:0"},config_dir="credentials",data_dir="router"},
    proxy={listen="127.0.0.1:${port}",cwd=".",key_env="CARTRIDGE_PROXY_KEY"},
    mcp={cwd="."},policy={default="ask"},harness={max_bytes=262144},gitfs={store_dir="gitfs"}}`);
  return [binary, "--dir", builtin, "--profile", directory];
}

async function run(args: string[], cwd: string, input = "") {
  const child = Bun.spawn(args, { cwd, stdin: new Blob([input]), stdout: "pipe", stderr: "pipe" });
  const timeout = setTimeout(() => child.kill(), 30_000);
  try {
    const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    expect(code, stderr).toBe(0); return stdout;
  } finally { clearTimeout(timeout); if (child.exitCode === null) child.kill(); }
}

for (const name of ["policy", "mcp", "proxy"]) {
  if (selected !== "all" && selected !== name) continue;
  test(`${name}: real isolated protocol`, async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), `cartridge-${name}-smoke-`));
    try {
      if (name === "policy") {
        const command = profile(root, name);
        for (const [tool, decision] of [["read", "allow"], ["docs", "allow"], ["write", "ask"], ["unknown-tool", "ask"]]) {
          const out = await run([...command, "run", "policy", JSON.stringify({ tool, input: {}, context: {} })], root);
          expect(JSON.parse(out).decision).toBe(decision);
        }
        expect(JSON.parse(await run([...command, "run", "policy", "null"], root)).decision).toBe("deny");
      } else if (name === "mcp") {
        const messages = [
          { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "cartridge-development-smoke", version: "1" } } },
          { jsonrpc: "2.0", method: "notifications/initialized" },
          { jsonrpc: "2.0", id: 2, method: "tools/list", params: {} },
        ];
        const out = await run([...profile(root, name), "mcp"], root, messages.map(m => JSON.stringify(m) + "\n").join(""));
        const replies = out.trim().split("\n").map(line => JSON.parse(line));
        expect(replies.find(r => r.id === 1).error).toBeUndefined();
        const tools = replies.find(r => r.id === 2).result.tools;
        expect(tools.length).toBeGreaterThan(0);
        expect(tools.every((tool: any) => tool.name && tool.inputSchema)).toBe(true);
      } else {
        const port = await new Promise<number>((resolve, reject) => {
          const socket = net.createServer(); socket.once("error", reject);
          socket.listen(0, "127.0.0.1", () => { const port = (socket.address() as net.AddressInfo).port; socket.close(() => resolve(port)); });
        });
        const key = randomBytes(32).toString("hex");
        const child = Bun.spawn([...profile(root, name, port), "daemon"], { cwd: root, env: { ...process.env, CARTRIDGE_PROXY_KEY: key }, stdout: "ignore", stderr: "pipe" });
        const stderr = new Response(child.stderr).text();
        try {
          let ready = false;
          for (let i = 0; i < 300; i++) {
            if (child.exitCode !== null) throw Error(await stderr);
            try { expect((await fetch(`http://127.0.0.1:${port}/development-probe`, { signal: AbortSignal.timeout(1000) })).status).toBe(401); ready = true; break; }
            catch { await Bun.sleep(100); }
          }
          expect(ready).toBe(true);
          expect((await fetch(`http://127.0.0.1:${port}/development-probe`, { headers: { Authorization: "Bearer " + key }, signal: AbortSignal.timeout(3000) })).status).toBe(404);
        } finally { child.kill("SIGTERM"); const timeout = setTimeout(() => child.kill("SIGKILL"), 3000); await child.exited; clearTimeout(timeout); }
      }
    } finally { fs.rmSync(root, { recursive: true, force: true }); }
  }, 90_000);
}
