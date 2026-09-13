import { expect, test } from "bun:test";
import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import { observationFixture, secret } from "./observation-fixture";
const binary=process.env.CARTRIDGE_TEST_BIN;
if(!binary)throw Error("CARTRIDGE_TEST_BIN must name the freshly built runtime");
test("actual native dispatch attributes mixed work, revisions and uncertain completions without bodies",async()=>{
 const fixture=observationFixture(binary!);
 try{
  const actions=[{op:"describe"},{op:"call",input:{mode:"success",secret},context:{actor:"ui",source:"poller"}},{op:"call",input:{mode:"bump"}},{op:"describe"},{op:"call",input:{mode:"failure"}},{op:"call",input:{mode:"transport"}},{op:"call",input:{mode:"invalid"}},{op:"call",input:{mode:"interrupted"}},{op:"cancel"}];
  const result=await fixture.run("agent",actions);expect(result).toHaveLength(actions.length);expect(result[1].data.content).toBe(secret);expect(result[5].error).toContain(secret);
  await fixture.run("ui",[{op:"call",input:{secret}}]);await fixture.run("poller",[{op:"call"}]);await fixture.run("unknown",[{op:"call",context:{actor:"agent"}}]);
  const rows=fixture.rows(), completions=rows.filter(r=>r.stage==="completion");expect(rows).toHaveLength(24);
  expect(completions.slice(0,9).map(r=>r.outcome)).toEqual(["described","success","success","described","tool_error","transport_error","invalid_response","interrupted","cancel_acknowledged"]);
  expect(completions.slice(0,9).map(r=>r.completion_known)).toEqual([true,true,true,true,true,false,false,false,false]);
  expect(completions[0].activity).toBe("discovery");expect(completions[1].actor).toBe("agent");expect(completions[1].activity).toBe("deliberate");
  expect(completions[9].actor).toBe("ui");expect(completions[9].activity).toBe("read");expect(completions[10].activity).toBe("poll");expect(completions[11].actor).toBeNull();
  expect(completions[0].descriptor_revision).toHaveLength(64);expect(completions[1].descriptor_revision).toBe(completions[0].descriptor_revision);expect(completions[3].descriptor_revision).not.toBe(completions[0].descriptor_revision);expect(completions[4].descriptor_revision).toBe(completions[3].descriptor_revision);
  expect(completions[0].provider_generation).toBe(completions[4].provider_generation);expect(completions[9].provider_generation).not.toBe(completions[0].provider_generation);expect(completions[9].descriptor_revision).toBeNull();
  expect(completions[1].response_bytes).toBe(Buffer.byteLength(JSON.stringify(result[1].data)));expect(completions[1].elapsed_ms).toBeGreaterThanOrEqual(0);
  expect(JSON.stringify(rows)).not.toContain(secret);expect(rows.every(r=>r.input===undefined&&r.content===undefined&&r.result===undefined)).toBe(true);
 }finally{fixture.close();}
},20000);
test("disabled, failed and rotating diagnostic sinks preserve native tool replies",async()=>{
 const fixture=observationFixture(binary!);
 try{
  expect((await fixture.run("agent",[{op:"call"}],{enabled:false}))[0].data.content).toBe(secret);expect(fixture.rows()).toHaveLength(0);
  expect((await fixture.run("agent",[{op:"call"}],{diagnostic:join(fixture.dir,"missing","log")}))[0].data.content).toBe(secret);
  await fixture.run("agent",Array.from({length:30},()=>({op:"call"})),{cap:2048});expect(existsSync(fixture.diagnostic+".1")).toBe(true);expect(statSync(fixture.diagnostic).size).toBeLessThanOrEqual(2048);expect(statSync(fixture.diagnostic+".1").size).toBeLessThanOrEqual(2048);expect(fixture.rows().length).toBeLessThan(60);
 }finally{fixture.close();}
},20000);
test("native provider exits after a partial reply without inventing completion",async()=>{
 const fixture=observationFixture(binary!);
 try{const result=await fixture.run("agent",[{op:"call"}],{partial:true,allowFailure:true});expect(result.exit).toBe(1);const rows=fixture.rows();expect(rows.some(r=>r.stage==="attempt")).toBe(true);const row=rows.find(r=>r.stage==="completion");if(row){expect(row.outcome).toBe("transport_error");expect(row.completion_known).toBe(false);expect(row.dispatched).toBe(true);expect(row.response_bytes).toBeNull();}expect(rows.some(r=>r.completion_known===true)).toBe(false);}finally{fixture.close();}
},15000);
