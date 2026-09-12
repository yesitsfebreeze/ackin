---
kind: work
description: "Offline end-to-end coding-agent composition"
status: done
needs:
  - "[[lua-only-core-boundary]]"
  - "[[fs-tool-plugin-file-search]]"
  - "[[shell-tool-plugin]]"
  - "[[policy-plugin]]"
  - "[[session-transcript-durability-run-checkpoint]]"
  - "[[harness-plugin]]"
  - "[[safe-transcript-compaction]]"
  - "[[agent-loop-plugin]]"
  - "[[memo-tool-plugin]]"
  - "[[streaming-agent-output]]"
  - "[[terminal-agent-client]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Composing the coding profile or verifying tools, approvals, persistence and streaming together without live credentials"]
---

# coding-profile-integration

## Do

Own final composition in `.zirkle/default/init.lua` and config.lua, plus justfile launch/test targets. Keep existing router/sessions behavior while adding Lua entries for fs, shell, policy, harness, memo and the separate agent process. The terminal remains a socket client, not agent orchestration. Define one exact tools table in init.lua, pass it to agent entry.inject and config.tools, and retain static agent dependencies. Harness receives descriptors from agent rather than maintaining a second tool list. Plugin binaries own their workspace memberships in their respective PRDs; this item verifies the final workspace instead of independently adding duplicate members.

Default policy allows reads and asks before mutations, shell and memo writes. Include configured model choice, request/step/output limits and compaction settings with safe defaults. A separate isolated test profile uses a fake router and fake kern adapter peer; it never reads personal credentials or mutates the real project record. No live provider request is part of acceptance. Every PRD in this coding-agent milestone is a dependency of this integration terminal.

This memo owns only the bounded unit below; linked prerequisites own their implementations. Agent separation follows [[agent-is-a-separate-plugin]].

## Spec

Planning status: proposed specification based on source inspection. No implementation probe or code tests have run. Keep every acceptance box unchecked until verified. File footprint: `.zirkle/default/init.lua`, `.zirkle/default/config.lua`, `justfile`, `core/tests/`, `Cargo.toml`.

Wire the complete default profile through Lua paths only. Agent binary remains independently replaceable by changing its Lua wrapper/profile entry, without rebuilding glue, fs or harness. Register memo alongside file and shell tools. Make the launch recipe build all required binaries, launch the same profile the terminal connects to, and clean up daemon/tool children on exit. Keep stdout JSON protocol channels separate from diagnostics.

Create an offline integration test/profile under existing core tests. Use temporary workspace and XDG roots, fake router Chat/SSE responses and a fake kern MCP child. Drive a user task that reads a file, requests an edit, denies it once, then approves a second proposal and verifies changed bytes plus sessions touched files. Exercise a shell call, memo read/write policy split, final answer, persisted transcript restart, an interrupted tool without replay, compaction and streaming disconnect. Use small deterministic fixtures rather than broad infrastructure. Add `just agent-smoke` for this test; do not introduce live-network gates.

Capture event IDs, approval outcomes, file bytes and journal contents in assertions. Prove omission/replacement of a tool only changes profile keys/descriptors; unknown requested tools never fall through to arbitrary service lookup. Run final existing workspace gates after the focused smoke passes.

## Check

- [x] Default profile contains only path entries and independently registered agent, harness, policy, tool, router and sessions components.
- [x] One tools table controls effective agent injections, descriptors and dispatch; removing a key prevents dispatch without an agent code change.
- [x] Offline read/edit task denies first mutation, approves second, persists transcript and records touched file correctly.
- [x] Shell timeout/cancel, memo read/write classification, compaction and truncated stream paths finish without replay or child leaks.
- [x] Restart preserves completed conversation and exposes interrupted work without reexecuting tools.
- [x] just agent-smoke, just check and just test pass without model credentials, a live kern daemon or personal record access.

Verification commands (future implementation gate; not run during planning):

```sh
set -eu
just agent-smoke
test -f .zirkle/default/init.lua
just check
just test
```

## Result

2026-09-10 08:00 — coding-profile-integration landed 20ce945..e652eec; composed default profile, one tools table, 7-test offline smoke, just agent-smoke green
