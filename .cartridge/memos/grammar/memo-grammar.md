---
kind: grammar
description: "the record, its service and its cartridge: one word for everything that is written down"
overloads: one record file, the memo service and tool, the memo cartridge, a countable memo
date: "2026-09-12"
---

# memo

Four senses. One **memo** is one record file: `.cartridge/memos/<kind>/<name>.md`
(or `@<cartridge>/...` when a cartridge ships it). The **memo** service and
its agent tool `tool.memo` are how anything reaches the record. The **memo**
cartridge (`builtin/memo`) owns the record, its validation and the resolver.
And prose says "a memo" as a count, not a proper noun. Authority is
[[type]] for the file contract and `builtin/memo/README.md` for the service.

The bite: "memo" alone is rarely ambiguous — the trap is the neighbour word
**memory**, a different cartridge, store and tool surface entirely. Read
`memory-grammar` before naming anything storage-flavoured; the record itself
is [[record-grammar]].