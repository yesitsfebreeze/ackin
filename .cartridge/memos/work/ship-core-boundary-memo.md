---
kind: work
description: Ship the core-boundary concept memo with the default record
status: done
owner: claude
uses:
  - usage: "[[read-usage]]"
    when: ["understanding what was shipped to define the core, verifying the core concept memo"]
---

## Outcome

Every zirkle install's record answers "what belongs in the core" without reading
Rust: a shipped `kind: note` at `@memo/note/core.md` distilling the recorded
core boundary.

## Spec

The note is `kind: note`, shipped under `builtin/memo/.zirkle/memos/note/core.md`,
with `uses` matching architecture and boundary questions. Content: core owns
composition/effects (`runtime.rs`, `fiber.rs`), per-entry memory and service
references (`memory.rs`, `service.rs`), cartridge binding (`lua.rs`,
`context.rs`, `loader.rs`), process/socket wire (`cartridge.rs`, `sdk.rs`,
`socket.rs`) and the narrow CLI launch branch (`main.rs`). The membership test:
"getting it wrong corrupts state or moving it needs a host restart". Product
behavior (agent, tools, policy, routing, persistence, rendering) is cartridge
behavior. Links `[[plugin]]` and `[[program-map]]`, and the recorded decision
`[[core-composes-and-the-cli-selects-services]]`.

## Check

- [x] `builtin/memo/.zirkle/memos/note/core.md` ships with the memo cartridge record.
- [x] Distills `decision/core-composes-and-the-cli-selects-services.md` without contradicting it.
- [x] Linking valid in the merged record ([[plugin]], [[program-map]] resolve).

## Approach

1. Author the shipped note distilling the recorded boundary.
2. Wire [[core]] into `@memo/note/program-map.md`.
3. Verify shipped record semantics (`cargo test -p memo`) and resolve.

## Result

Landed 2026-09-12. `builtin/memo/.zirkle/memos/note/core.md` authored and wired into
program-map. Verified: `cargo test -p memo_cartridge` 29/29; real-profile
composition `cargo nextest run -p zirkle -E 'test(profile)'` 11/11; real-host
`zirkle call memo` resolve ranks `@memo/note/core.md` first for a core/boundary
query.
