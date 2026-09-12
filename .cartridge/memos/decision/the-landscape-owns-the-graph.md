---
kind: decision
description: "The composition view leaves core and lives in the landscape: one graph over every registered cartridge's memos, tools and capabilities, served by the memo tool's landscape op"
status: accepted
date: "2026-09-12"
uses:
  - usage: "[[read-usage]]"
    when: ["extending the graph, the composition projection, or agent discovery over the composition"]
---

# the-landscape-owns-the-graph

## Decision

`core/landscape.rs` keeps only the raw read-only composition snapshot; the
view over it and the git tracked-files dependency move out of core into the
`landscape` crate (`builtin/landscape`, renamed from `toolgraph` on
2026-09-12). The landscape is the one graph over every registered cartridge:
composition, generations and dependencies from the host snapshot; memos from
the record; tools and their `describe` schemas; capabilities and services from
`provide`/`inject`; and the uses the resolver journal counts. `builtin/memo`
links the landscape in-process and serves it as one `landscape` operation — no
separate process cartridge, no second store, and nothing named inventory.

## Why

The composition view existed to serve `tool.memo`'s discovery operation — an
agent feature, not runtime semantics — and shelling out to `git ls-files`
put a workspace assumption inside core. The landscape engine and the
resolver's ranking machinery already live in the crate, so the projection
joins them: one place answers what exists, what it provides and what gets
used. The killed-analyst memo stays true: the analyst is dead, the surface
it probed survives as the landscape.

The point of one graph: when work starts, the agent looks up the landscape
and loads only what the job needs — memos, tools, capabilities — instead of
composing everything into context.

## Consequences

`landscape::view` owns the git join, the snapshot digest and cursor paging;
`landscape::surface::compose` grows the graph from the three sources of truth —
the composition entries, every `tool.*` key the profile injects into `memo` as
it describes itself now, and the record's memos read without the bootstrap or
the writer lock — and the observation journal is what it ranks by. The memo
tool's one `landscape` op serves both halves: rows and graph counts always, and
with a `query`, ranked hits naming what each one connects to. `sdk::Host::landscape`
is the composition query the host serves to cartridges; nothing is called an
inventory. The engine's duplicate copy of the resolver machinery went with the
rename: the live resolver stays in `builtin/memo`. The client surface stays in
core, per [[the-client-surface-is-a-bridge]].
