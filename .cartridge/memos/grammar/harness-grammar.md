---
kind: grammar
description: "the harness cartridge and the harness concept — the thing that composes the prompt and renders the request"
overloads: the harness cartridge, the harness as runtime concept, third-party harnesses
date: "2026-09-12"
---

# harness

Three senses. The **harness** cartridge (`builtin/harness`) composes the
system prompt from enabled `kind: system` memos, renders the session
placeholders, and serves `harness.selftest` and `harness.integration`. The
**harness** as a concept is that runtime role: whatever composes context,
dispatches tools and feeds results back — the proxy cartridge executes
harness tools, so prose says "the harness" meaning the role, not always the
cartridge. Third-party **harnesses** (Claude Code and its kin) appear in
migrated notes as consumers of `cartridge mcp`. Authority is
`builtin/harness/README.md` and [[system]] for composition.

The bite: "harness" alone does not say cartridge or role. The cartridge is
the implementation; the role spans proxy and agent too. The bridge between
cartridge's harness and third-party ones is its own word — see
[[bridge-grammar]].