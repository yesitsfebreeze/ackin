---
kind: note
description: "Shared JSON conventions for separately implemented coding plugins"
uses:
  - usage: "[[read-usage]]"
    when: ["Implementing sessions, harness, tools, policy, agent, or the coding profile"]
    tags: [documentation]
---

# coding-plugin-contracts

These conventions fill unspecified field names in the existing work contracts; their acceptance requirements still govern. Tool descriptors use bare names (`read`, `bash`, `memo`) mapping to exactly `tool.<name>`. Configured injection/dispatch lists contain full keys. Policy receives the bare descriptor name as `tool`.

Tool call: `{op:"call",context:{session,run,call,cwd},input:{...}}`. Result: `{content:string,error:boolean}`. Cancellation uses `{op:"cancel",context:{session,run,call,cwd}}` on the same key. The invocation `call` is distinct from the provider's `tool_call_id`. Only orchestration supplies context; input schemas never advertise context or cancellation fields. Cancellation before registration must remain effective; bounded tombstones must fail closed rather than evict protection for live runs. Do not invent a separate tool.cancel service.

Session checkpoint: `{op:"checkpoint",id,expected_revision,records:[...],agent:{...}}`, returning `{revision,transcript}`. Transcript is a persistent numeric buffer ID. Session `get` exposes both, plus `agent`. Agent metadata is shallow-merged. Message records use `{v:1,kind:"message",run,message:<Chat message>}`. Lifecycle records share `v`, `kind`, and `run`: `run_started` carries `model`; `tool_started` carries `call`, `tool`, `tool_call_id`, `input`; `tool_finished` carries `call`, `tool`, `tool_call_id`, `result`; `run_finished` carries `phase` and optional `error`; `model_turn_started` and `model_turn_finished` carry turn diagnostics. Sessions validates storage shape, not Chat pairing: records of a kind it does not know are stored as-is when `v`, `kind` and `run` are present; harness and agent validate pairing.

Persisted agent fields are `run`, `phase`, `step`, `error`, and `pending`. Phase is `running`, `awaiting_approval`, `completed`, `failed`, `cancelled`, or `interrupted`; active phases prevent cwd changes. Clear stale values explicitly with null because checkpoint merge is shallow. Pending approval is an immutable `{session,run,call,tool,input}` object. Agent status exposes `{session,run,phase,step,error,pending}`. A worker may add fields needed for recovery while preserving these conventions.

Harness context request is `{op:"context",session,descriptors}`; response `{system,messages,tools}`. Descriptors are raw; output tools are Chat function objects with `parameters:input_schema`. Agent prepends the returned system text once.

Agent start `{op:"start",session?,prompt,model?}` returns `{session,run}` promptly after initial persistence. Status/cancel use `{op,session,run?}`; answer uses `{op:"answer",session,run,call,decision:"allow"|"deny"}`. Policy request `{tool,input,context}` returns `{decision:"allow"|"deny"|"ask",reason?}` synchronously. Missing policy blocks execution.

Outward events use `{event:"agent",data:{session,run,seq,kind,...}}`. `seq` increases per run. Approval events use kind `approval_requested` and `pending`; tool events use `tool_started`/`tool_finished` with `call` and `tool`, omitting full input. Final `text` events carry `text`; terminal events use `done` for completed and `error` otherwise, with `phase` and optional `error`. Checkpoints precede events; later streaming deltas are explicitly provisional. Direct SDK `host.send("agent", data)` produces the event envelope.

Only coding-profile integration owns the final profile. One ordered full-key tools table supplies agent `entry.inject` and `config.tools`. Harness has no independent tool registry. Every Cargo/just build in a worker uses a lane-specific `CARGO_TARGET_DIR` because this machine otherwise resolves every lane to the trunk build directory.
