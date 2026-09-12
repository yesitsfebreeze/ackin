---
kind: grammar
description: "the one graph over every registered cartridge: the crate, the engine, the composition view, the memo op that serves both"
overloads: the landscape crate, the graph engine, the composition view, the former toolgraph and the former inventory op
date: "2026-09-12"
---

# landscape

Three senses, one thing. The **landscape** is the graph over every registered
cartridge — memos, tools, capabilities, services, and the uses the resolver
journal counts. The **landscape** crate (`builtin/landscape`, renamed from
`toolgraph` on 2026-09-12) is the ranking engine and that graph. **`landscape`**
is the memo tool operation that serves both halves: `landscape::view` projects
the composition core snapshots, and `landscape::surface::compose` grows the graph
over it. There is no separate landscape process or tool — `builtin/memo` links
the crate in-process — and nothing is called an inventory any more: the word was
a fourth name for the same thing (the router's provider inventory is unrelated).
Authority is [[the-landscape-owns-the-graph]] and
`builtin/landscape/src/lib.rs`.

The bite: readers reach for a "sight" or "toolgraph" tool that does not exist.
The landscape has no key of its own — it is reached through `tool.memo`. Sits
next to [[record-grammar]]; whether it should stay behind that one door is
[[streamline-the-overloaded-names]].