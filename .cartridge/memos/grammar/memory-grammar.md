---
kind: grammar
description: "the vector-memory cartridge: standalone engine with its own store, not the memo record"
overloads: the memory cartridge, its .cartridge/memory store, the per-profile memory bank
date: "2026-09-12"
---

# memory

Three senses. The **memory** cartridge (`builtin/memory`) is the standalone
memory engine: ingest, embed, index, retrieve with provenance, a CLI and a
daemon of its own. Its store is `.cartridge/memory/`. Separately, each profile
entry has a core-resident **memory bank** with a versioned snapshot,
retained across replacements — nothing to do with the cartridge. Authority is
`builtin/memory/README.md` for the engine and `README.md` for the bank.

The bite: nothing in cartridge is called "memory" in the record sense — that is
[[memo-grammar]] and [[record-grammar]]. A sentence like "store it in memory"
is wrong twice over: say the record or the store. This word is the top
rename candidate; see [[streamline-the-overloaded-names]].