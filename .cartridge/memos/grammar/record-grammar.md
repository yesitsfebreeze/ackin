---
kind: grammar
description: "the active memo record in .cartridge/memos: the source of truth the landscape projects, not the memory store"
overloads: the memo record, the record as the graph's source of truth, a cartridge's shipped record
date: "2026-09-12"
---

# record

Three senses. **The record** is the active memo record: `.cartridge/memos/`, owned
by the memo cartridge, one Markdown file per memo. A **shipped record** is a
cartridge's own `.cartridge/memos/`, merged read-only into the workspace record.
And in the landscape's prose, "the record" is one of the three sources of
truth the graph projects — with the tool inventories and the observation
journal — so "the record" can mean the whole body of memos, not one folder.
Authority is [[type]] for the file contract,
[[the-landscape-owns-the-graph]] for the projection, `builtin/memo/README.md`
for the service.

The bite: "record" never means the memory engine's store — that is
`memory-grammar` — and never the per-profile memory bank. Say "the record"
for memos, "the store" for embedded documents, "the bank" for the
core-resident snapshot. See [[streamline-the-overloaded-names]].