---
kind: grammar
description: "the bridge: the thin client surface where third-party agents drive cartridge — config key, test lane, decision"
overloads: the bridge config key, the client surface as bridge, the bridge test lane
date: "2026-09-12"
---

# bridge

Three senses, one decision. **The bridge** is the client surface: third-party
agents (Claude Code and its kin) driving cartridge through `cartridge mcp` and
`cartridge call`, instead of cartridge's own UI driving them. The UI's **`bridge =
true`** config turns that surface on for a profile. **`core/tests/bridge.rs`**
is the test lane for it — renamed from `ui.rs` when the surface became the
bridge. Authority is the decision [[the-client-surface-is-a-bridge]] and
[[harness-grammar]] for what stands on each end.

The bite: "bridge" is not a component you can point at — no cartridge or
service is named bridge. It is a role: cartridge's record and shell served to an
agent that brings its own model. If you cannot say which end of the bridge
you mean, you mean the decision, not the code.