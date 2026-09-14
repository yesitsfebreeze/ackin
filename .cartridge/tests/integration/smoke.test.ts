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

/** An isolated copy of the one shipped profile: its `.cartridge/init.lua` under
 *  a scratch root, with stores, ports and boards pointed at that root. There is
 *  no profile to select on the command line, so the host is run with the scratch
 *  root as its working directory and reads the `.cartridge` beside it. */
function profile(root: string, name: string, port = 0) {
  const directory = path.join(root, name), builtin = path.join(directory, "builtin");
  const inner = path.join(directory, ".cartridge");
  fs.mkdirSync(builtin, { recursive: true });
  fs.mkdirSync(inner, { recursive: true });
  const init = name === "policy" ? 'return {{id="policy",path="policy"}}\n'
    : fs.readFileSync(path.join(runtime, ".cartridge", "init.lua"), "utf8");
  fs.writeFileSync(path.join(inner, "init.lua"), init);
  // Shallowest path first: a cartridge that sits inside another one (`live/mcp`)
  // arrives through its parent's link and must not be linked over it.
  const modules = [...new Set([...init.matchAll(/path\s*=\s*"([^"\n]+)"/g)].map(match => match[1]))]
    .sort((a, b) => a.split("/").length - b.split("/").length);
  for (const module of modules) {
    const destination = path.join(builtin, module);
    if (fs.existsSync(destination)) continue;
    if (name === "proxy" && module === "router") {
      fs.mkdirSync(destination, { recursive: true });
      fs.writeFileSync(path.join(destination, "cartridge.json"), JSON.stringify({ name: module, entry: "init.lua" }));
      fs.writeFileSync(path.join(destination, "init.lua"), 'return {provide={"router"},apply=function(ctx) ctx:provide("router",function() return {data={}} end) end}');
    } else {
      fs.mkdirSync(path.dirname(destination), { recursive: true });
      fs.symlinkSync(fs.realpathSync(path.join(runtime, "builtin", module)), destination);
    }
  }
  const prdRoot = path.join(directory, "prd-fixture");
  fs.mkdirSync(path.join(prdRoot, ".cartridge/boards/root"), { recursive: true });
  fs.writeFileSync(path.join(prdRoot, ".cartridge/boards/root/settings.md"), "# Isolated smoke board\n");
  fs.writeFileSync(path.join(inner, "config.lua"), `return {
    sessions={dir="sessions"},memory={dir="memory",reason={url=""},tick={interval_secs=0},queue={enabled=false}},
    prd={root=${JSON.stringify(prdRoot)},default_board="root"},
    router={listen={"127.0.0.1:0"},config_dir="credentials",data_dir="router"},
    proxy={listen="127.0.0.1:${port}",cwd=".",key_env="CARTRIDGE_PROXY_KEY"},
    live={port=0,dir="live-data"},["live-record"]={dir="live-data"},workspace={dir="workspace-pages"},
    mcp={cwd="."},policy={default="ask"},harness={max_bytes=262144},gitfs={store_dir="gitfs"}}`);
  return { command: [binary, "--dir", builtin], cwd: directory };
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
        const { command, cwd } = profile(root, name);
        for (const [tool, decision] of [["read", "allow"], ["docs", "allow"], ["write", "ask"], ["unknown-tool", "ask"]]) {
          const out = await run([...command, "run", "policy", JSON.stringify({ tool, input: {}, context: {} })], cwd);
          expect(JSON.parse(out).decision).toBe(decision);
        }
        expect(JSON.parse(await run([...command, "run", "policy", "null"], cwd)).decision).toBe("deny");
      } else if (name === "mcp") {
        const messages = [
          { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "cartridge-development-smoke", version: "1" } } },
          { jsonrpc: "2.0", method: "notifications/initialized" },
          { jsonrpc: "2.0", id: 2, method: "tools/list", params: {} },
        ];
        const { command, cwd } = profile(root, name);
        const out = await run([...command, "mcp"], cwd, messages.map(m => JSON.stringify(m) + "\n").join(""));
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
        const { command, cwd } = profile(root, name, port);
        const child = Bun.spawn([...command, "daemon"], { cwd, env: { ...process.env, CARTRIDGE_PROXY_KEY: key }, stdout: "ignore", stderr: "pipe" });
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
