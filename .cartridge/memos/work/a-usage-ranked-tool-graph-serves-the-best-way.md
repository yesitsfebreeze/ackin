---
kind: work
description: "Tooling use, memos and routines are ranked in one searchable graph; how often a thing is used enforces its ranking"
status: open
level: 9
priority: P1
subwork: ["[[tool-graph-engine-is-the-ranking-database]]", "[[the-resolver-runs-on-the-tool-graph-engine]]", "[[tool-dispatch-and-routines-are-graph-nodes]]", "[[agents-query-the-tool-graph]]"]

---

# a-usage-ranked-tool-graph

## Do

The ranking machinery inside the record — the memo resolver's usage matching,
observation events and usefulness feedback, and the memory crate's graph and
retrieval work — is scattered across cartridges. It should be its own tool: a
ranking database whose nodes are everything the system can reach for — tools,
memos, routines — with the connections between them.

How often a thing is used enforces its ranking: every dispatch, resolve and
outcome is an observation the graph counts. Searching it returns the best
tools and the best way to do things, with descriptions, consistently — not
per-session memory, one ranked surface that any dispatched agent consults
through the same tool.

Extraction means: the existing resolver behaviour (usage `when:` matching,
provenance, feedback events, [[record-resolver-usefulness-feedback]]) keeps
working while its engine moves behind one service; the memo record, tool
dispatch and routines all consult it; and a query like "best tool for X" or
"how we do Y" answers from the graph, ranked by observed use.

## Check

- [ ] [[tool-graph-engine-is-the-ranking-database]] is done.
- [ ] [[the-resolver-runs-on-the-tool-graph-engine]] is done.
- [ ] [[tool-dispatch-and-routines-are-graph-nodes]] is done.
- [ ] [[agents-query-the-tool-graph]] is done.

## Probe

Analyst probe, 2026-09-12, committed on lane
`work/a-usage-ranked-tool-graph-serves-the-best-way` (f34ac50): the
`builtin/toolgraph` engine crate — `Node` (kind, key, description, when,
tags), `search` ranking by field-weighted token match times observed standing
(`1 + ln(1 + uses)`), `counts_from_journal` with stage weights, and an ignored
`real_record_search` smoke over a real record. Unit tests prove kinds are
searched with descriptions and observed use outranks an equally matched
unused entry; clippy and the crate tests are green. The extraction seam was
measured: `usage.rs` and `resolver.rs` depend on the record's namespace
through one glob (`Memo`, `Input`, `workspace`, `no_symlink`, `atomic_write`,
`digest`, `Names`), and the compatibility bar is the existing
`builtin/memo/tests/` suite.

User request, 2026-09-12: "We need a tool graph: how often a tool is used
enforces its ranking. We can search for tools and we get descriptions. It's a
ranking database for tools, memos, and routines for everything that we have,
and it gives us back the best tools and the best way to do things in a
consistent manner."
