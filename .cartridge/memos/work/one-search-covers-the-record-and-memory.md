---
kind: work
description: "One ranked search answers what exists across the record and the memory store, instead of a tool per store"
status: open
level: 10
estimate: 4h
uses:
  - usage: "[[read-usage]]"
    when: ["looking something up before starting work, or extending what the landscape indexes"]
---

# one-search-covers-the-record-and-memory

## Outcome

One query returns the best matches across everything the system remembers: the
record's memos, the tools and cartridges the landscape already graphs, and the
memory store's facts. A caller asks once, with a situation in its own words, and
does not choose a store first.

The memory store keeps its own tool for writing and for reads that are about
memory as memory (ingest, entity lookup); what goes away is having to know which
of two searches to ask in order to find out that something exists.

Scope is retrieval. Merging the two stores is explicitly not the outcome: the
record stays the record, memory stays memory, and the graph stays derived from
both rather than becoming a third copy.

## Check

- [ ] `tool.memo {"op":"landscape","query":"<a phrase only a memory fact
      matches>"}` returns that fact as a hit, with its kind naming the memory
      store and a reference the caller can read it back by.
- [ ] The same call still returns memo, tool and cartridge hits for phrases they
      match, ranked against the memory hits by the same score and observed use.
- [ ] A profile without the memory cartridge answers the same query with the
      record and tool hits and no error — memory is an optional contributor, not
      a dependency of the search.
- [ ] Memory hits carry no content the caller did not ask for: the hit is a
      description and a reference, and reading the fact is a second call.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. [[the-memory-bank-is-wired-into-the-surface]] is done, so
`memory` and `memory-tool` are composed and `tool.memory` exists — but the
landscape graphs cartridges, tools and memos only
(`builtin/landscape/src/surface.rs`), so the two stores are two separate
searches and a caller has to guess which one holds what it needs.

The shape already fits: `Node` carries `kind`, `key`, `description`, `when` and
`tags`, and `compose` takes its sources as flat JSON rows, so memory
contributes rows the same way the record does. `builtin/memo`'s `landscape` op is
where they meet; it already describes every injected `tool.*` key, so reaching
the memory store is an existing dependency rather than a new one.

Two things to settle while building: memory rows are potentially many, so the
graph needs a bound (the record's scan is already capped — see
`MAX_MEMOS`/`MAX_RECORD_BYTES`), and a memory hit's `key` must be a reference
`tool.memory` accepts, so a hit leads to a read without the caller constructing
a query. Sits under the same ranking the resolver's observation journal feeds, so
a memory fact that gets used outranks one that never does.

Related: [[agents-query-the-tool-graph]] is the caller's side of this (active,
owned elsewhere) — this memo is what it has to search.
