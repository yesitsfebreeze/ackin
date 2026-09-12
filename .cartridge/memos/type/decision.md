---
kind: type
type: decision
description: A settled choice, its reasons, and the consequences to preserve
uses:
  - usage: "[[read-usage]]"
    when: [record a settled choice]
  - usage: "[[compose-usage]]"
    when: [writing a decision memo, record a settled choice]
---

# Decision

One decision memo records one choice that future work should follow. Name it
for the choice. Record decisions whose reasons would otherwise be lost; routine
implementation details can stay with the work. An unresolved choice belongs in
a question memo.

## Fields

Every instance has `kind: decision`, a nonempty `description`, `status`, and
`date` (a quoted ISO date, YYYY-MM-DD). Status is `accepted` or `superseded`.
Optional `supersedes` links to an earlier decision this choice replaces.
Optional `decided_by` identifies who made the choice when authority matters.
Do not attribute approval to a person who has not given it.

## Body

- `## Choice`: the actual rule or choice, including where it applies.
- `## Why`: the constraints and evidence that led to it. Mention meaningful
  alternatives only when they explain the choice.
- `## Consequences`: what this enables, costs, or requires next. Link to work
  for changes still to be implemented.

A decision being accepted does not mean its implementation is complete.
When changing the choice, write a new accepted decision with `supersedes`, then
mark the old one superseded and link to its replacement in its body. Preserve
the old rationale. Correct factual errors openly rather than rewriting the
history of who chose what.

## Starting shape

```markdown
---
kind: decision
description: Store session drafts alongside session state
status: accepted
date: "2026-09-10"
---

## Choice

Persist each draft in its session record.

## Why

The draft has the same ownership and lifetime as the session.

## Consequences

Session writes must preserve drafts until submission or explicit clearing.
```

These fields and lifecycle rules guide authors; the cartridge does not enforce
decision authority or supersession semantics.
