import { mkdtempSync, writeFileSync, readFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
export const secret = "OBSERVATION_FIXTURE_PRIVATE_BODY";
const caller = String.raw`
import { createInterface } from "node:readline";
const send=(x)=>process.stdout.write(JSON.stringify(x)+"\n");
if(process.argv.includes("hello")){send({inject:["tool.fixture"],provide:["fixture.run"]});process.exit();}
let next=100;const pending=new Map();
const call=(args)=>new Promise(resolve=>{const id=next++;pending.set(id,resolve);send({id,call:"tool.fixture",args});});
createInterface({input:process.stdin}).on("line",async line=>{
 const m=JSON.parse(line);
 if(m.apply){send({provide:"fixture.run"});send({ready:true});}
 else if(m.reply){pending.get(m.reply)?.(m);pending.delete(m.reply);}
 else if(m.call){const values=[];for(const action of m.args.actions) values.push(await call(action));send({reply:m.id,data:values});}
 else if(m.dispose)process.exit();
});`;
const partial = String.raw`
import {createInterface} from "node:readline";
const send=(x)=>process.stdout.write(JSON.stringify(x)+"\n");
if(process.argv.includes("hello")){send({provide:["tool.fixture"]});process.exit();}
createInterface({input:process.stdin}).on("line",line=>{const m=JSON.parse(line);if(m.apply){send({provide:"tool.fixture"});send({ready:true});}else if(m.call){process.stdout.write('{"reply":');process.exit(0);}else if(m.dispose)process.exit();});`;
const lua = `return {provide={"tool.fixture"},apply=function(ctx)
 local version=1
 ctx:provide("tool.fixture",function(args)
  if args.op=="describe" then return {name="fixture",description="version "..version,input_schema={type="object"}} end
  if args.op=="cancel" then return {cancelled=true} end
  local mode=args.input and args.input.mode
  if mode=="bump" then version=version+1 end
  if mode=="transport" then error("${secret}") end
  if mode=="invalid" then return {invalid="${secret}"} end
  if mode=="interrupted" then return {content="interrupted-outcome-unknown",error=true} end
  return {content="${secret}",error=mode=="failure"}
 end)
end}`;
export function observationFixture(binary: string) {
  const dir = mkdtempSync(join(tmpdir(), "native-observations-"));
  const diagnostic = join(dir, "diagnostics.jsonl");
  writeFileSync(join(dir, "caller.ts"), caller); writeFileSync(join(dir, "partial.ts"), partial);
  writeFileSync(join(dir, "caller.lua"), `return cartridge.process({${JSON.stringify(process.execPath)},${JSON.stringify(join(dir,"caller.ts"))}})`);
  writeFileSync(join(dir, "tool.lua"), lua);
  writeFileSync(join(dir, "partial.lua"), `return cartridge.process({${JSON.stringify(process.execPath)},${JSON.stringify(join(dir,"partial.ts"))}})`);
  return {
    dir, diagnostic,
    async run(source: string, actions: any[], options: { enabled?: boolean; cap?: number; diagnostic?: string; partial?: boolean; allowFailure?: boolean } = {}) {
      writeFileSync(join(dir, source+".lua"),readFileSync(join(dir,"caller.lua")));
      writeFileSync(join(dir, "init.lua"), `return {{id=${JSON.stringify(source)},path=${JSON.stringify(source+".lua")}},{id="provider",path=${JSON.stringify(options.partial?"partial.lua":"tool.lua")}}}`);
      const child = Bun.spawn([binary, "--dir", dir, "--profile", dir, "run", "fixture.run", JSON.stringify({ actions })], { env: { ...process.env, CARTRIDGE_TOOL_OBSERVATIONS: options.enabled === false ? "0" : "1", CARTRIDGE_DIAGNOSTICS: options.diagnostic || diagnostic, CARTRIDGE_DIAGNOSTICS_MAX_BYTES: String(options.cap || 8*1024*1024), CARTRIDGE_TOOL_ACTORS: JSON.stringify({ agent:{actor:"agent",activity:"deliberate"},ui:{actor:"ui",activity:"read"},poller:{actor:"background",activity:"poll"} }) }, stdout:"pipe", stderr:"pipe" });
      const [exit, stdout, stderr] = await Promise.all([child.exited, new Response(child.stdout).text(), new Response(child.stderr).text()]);
      if(exit) { if(options.allowFailure)return {exit,stdout,stderr}; throw Error(`runtime exit ${exit}: ${stderr}`); }
      return JSON.parse(stdout);
    },
    rows() { return [diagnostic+".1",diagnostic].filter(existsSync).flatMap(file=>readFileSync(file,"utf8").trim().split("\n").filter(Boolean).map(line=>JSON.parse(line))).filter(row=>row.msg==="tool_attempt"||row.msg==="tool_completion"); },
    close(){rmSync(dir,{recursive:true});},
  };
}
