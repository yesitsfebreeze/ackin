---
type: workflow
active_focus: []
priority_tags: []
auto_enqueue: true
min_sources_per_conclusion: 2
---

# Knowledge workflow — the loop's configuration

Read on every invocation: frontmatter, then `## Focus`, then `## Rules`.
The four frontmatter keys are the whole configuration `knowledge.py` reads.

## Focus

Topics, tags or folders the loop attends to. Empty means no filter; set,
`query` prefers the matching nodes.

- (add focus areas here)

## Rules

- Link every source to its conclusion with `[[wikilinks]]`.
- A conclusion saves only on `min_sources_per_conclusion` sources or more.
- Never enqueue a duplicate — `pending/` already holds it.
- `auto_enqueue: false` returns the gap and writes no pending file.
- Skip domains: (list domains to exclude)

## The loop

`query` against the record → a gap → `enqueue` the question, or research it
now → `remember` each finding as a source → `conclude` from two or more →
`relink` to hold the graph together. `pending/` at `priority: high` goes
first; `doctor` names a question older than thirty days, and one nobody
needed again is deleted, not drained.

`sources/.absorbed/` holds a source once a conclusion carries it under
`derived_from:` — moved there by hand, hidden from every query. Restore by
moving the file back to `sources/` and running `relink`.
