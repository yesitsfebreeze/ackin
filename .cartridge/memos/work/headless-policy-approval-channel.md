---
kind: work
description: A headless session needs an explicit path to policy-gated writes instead of a blanket denial
status: done
level: 10
estimate: 3h
---

## Outcome

Policy-gated writes (shell commands, memo validated writes) work from
headless sessions — proxy runs, `zirkle launch`, MCP clients — through an
explicit channel, rather than failing on every write while reads pass.

## Check

- [x] The shipped proxy profile allowlists write keys explicitly: with the
      `policy = { default = "ask", tools = { memo = "allow", shell = "allow" } }`
      now in `.zirkle/proxy/config.lua` active, `zirkle --profile <dir> run
      policy '{"tool":"memo","input":{"op":"write"},"context":{"session":"s","run":"r","call":"c","cwd":"/tmp"}}'`
      answers `allow`, the same request for a `shell` command answers `allow`,
      and an unlisted write (`memory` `ingest`) still answers `ask`.
- [x] `cargo test -p proxy -p mcp` passes, including
      `granted_dispatches_land_in_the_observation_journal_with_caller_and_turn`
      (proxy) and `granted_calls_land_in_the_observation_journal_with_caller_and_turn`
      (mcp): every granted dispatch reaches the `memo` `observe` op twice —
      `used`, then an `outcome` parented on it — under the dispatch's own
      session, run and call.
- [x] `cargo test -p proxy -p mcp` also passes
      `an_ask_denial_names_the_operation_and_the_missing_channel` in both
      crates: an `ask` decision still never executes or journals, and the
      error result names the operation and the missing channel (`approval
      required for memo write: this proxy/server has no interactive approval
      channel; allowlist the key in the profile policy to approve it`).

## Context

Observed 2026-09-12: the proxy profile runs `policy = { default = "ask" }`;
a headless proxy session has no approver, so every memo write and shell
command answered `approval required: this proxy has no interactive approval
channel`. Reads (memo, landscape, resolve, shell screen) worked; writes were
structurally impossible for the whole session. Options seen elsewhere:
Claude Code's classifier-reviewed auto mode, or an approval surface in the ui
cartridge answering pending policy prompts.

Pass one settled the design: the explicit channel is the profile's policy
allowlist — the human who launches a headless agent approves writes on the
client side, and the keys allowlisted in the profile execute without a
zirkle-side prompt. The interactive agent's approve path and journaling
contract are the pattern the headless surfaces mirror.

## Approach

Files:

- `builtin/proxy/service.rs` — `Tool` carries the descriptor digest; granted
  dispatches journal `used`/`outcome` through the host `memo` `observe` op
  (best-effort, origin session/run/call, `call`/`call:outcome` per stage,
  parent link, verdict from the result envelope); an `ask` denial names the
  operation and the missing channel.
- `builtin/proxy/Cargo.toml` — `sha2` for the descriptor digest.
- `builtin/proxy/tests.rs` — the fake answers `memo` `observe`; the two new
  tests above.
- `builtin/mcp/service.rs`, `builtin/mcp/Cargo.toml`, `builtin/mcp/tests.rs` —
  the same shape for MCP.
- `.zirkle/proxy/config.lua` — the explicit allowlist.
- `builtin/proxy/README.md`, `builtin/mcp/README.md` — the channel and the
  journaling documented.

The probe (pass one, committed on `work/headless-policy-approval-channel`)
already built and verified all of it:

1. `cargo test -p proxy -p mcp` — 21 tests green, including the four new
   ones (run again after the final fmt pass; only assertion reflows
   changed).
2. Real-binary probes with a minimal profile carrying the shipped policy
   expression: memo write and shell answer `allow`, `memory` `ingest` and
   `edit` answer `ask`; an allowlisted memo write saved
   `note/probe-granted-write.md` in a scratch record.
3. A real `memo` `observe` used/outcome pair in the proxy's exact request
   shape landed in `.zirkle/resolver/events.jsonl` and showed in the
   `resolver` report with caller and turn.

Left for the implementer:

1. Re-run the gates on a machine with disk headroom: `just check` and
   `just test` (the lane's isolated /tmp target kept hitting ENOSPC from
   concurrent agents' builds; the crates themselves compile warning-free).
2. Optionally one live e2e: `zirkle --profile proxy daemon` plus a launched
   agent writing a memo, then `zirkle --profile proxy run memo
   '{"cwd":"<repo>","op":"resolver"}'` to see the granted write's
   used/outcome pair under the proxy session.
3. Land.


## Result

Landed on `work/headless-policy-approval-channel`. The explicit headless
approval channel is the profile's policy allowlist: `.zirkle/proxy/config.lua`
ships `policy = { default = "ask", tools = { memo = "allow", shell = "allow" } }`,
and the proxy (`builtin/proxy/service.rs`) and MCP (`builtin/mcp/service.rs`)
journal every granted dispatch through the `memo` `observe` op — `used`, then an
`outcome` parented on it — under the dispatch's own session, run and call,
while an `ask` stays a denial that names the operation and the missing channel
(pass one, commit 99eae02; the READMEs of both crates document the channel and
the journaling). Acceptance checks verified and ticked on the lane (1a211f8),
and the full gates re-run with disk headroom: `just check` clean and `just test`
green end to end — all nine `core/tests` python files pass, after building every
workspace bin into the isolated target first (the tests resolve cartridge
executables beside the zirkle binary, so a fresh isolated target needs
`cargo build --workspace --bins` before the python phase). No live e2e through
a real model was run — it needs live credentials — the mechanics are covered by
the crate tests plus the real-binary policy probes above. Landing is left to
the coordinator: lane branches are landed serially, so the branch stops here.
