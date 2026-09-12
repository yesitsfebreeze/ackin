---
kind: type
type: work
description: A bounded outcome to deliver and the evidence that it is complete
uses:
  - usage: "[[read-usage]]"
    when: [planning a bounded outcome and acceptance checks]
  - usage: "[[compose-usage]]"
    when: [writing a work memo, planning a bounded outcome and acceptance checks]
---

# Work

One work memo owns one outcome. Name it for that outcome, with enough context
that another person can pick it up. Start with the request and a checkable
result; add implementation detail when it is known. Small work stays small.

## Fields

Every instance has `kind: work`, a nonempty `description`, and `status`:

- `open`: the outcome is wanted and nobody is currently implementing it.
- `active`: an identified owner is implementing it.
- `blocked`: progress requires something named in the Blocker section.
- `done`: the outcome is delivered and its checks have passed.
- `cancelled`: the outcome is no longer wanted; record why.

Optional `owner` identifies the person or session currently responsible; require
it while active and remove it when that ownership ends. A claim is not a lock.
Check existing ownership before taking work.

Optional `needs` is a YAML list of quoted wiki links to prerequisites: other
work must be done, questions answered, and decisions accepted. Optional
`subwork` lists child work whose outcomes together deliver this memo. Children
also count as prerequisites for completion. Keep these relationships acyclic.
A body link supplies context; it does not make a prerequisite.

## Body

- `## Outcome`: what must become true and why; include scope boundaries where useful.
- `## Check`: observable acceptance criteria as unchecked boxes. Commands,
  expected results, or a concrete manual observation make a criterion testable.
- `## Approach`: optional implementation steps, added once understood.
- `## Blocker`: required while blocked; name the obstacle and what clears it.
- `## Result`: required when done or cancelled; record delivered behavior and
  verification evidence, or the reason for cancellation.

Check a box only after observing its result. A commit alone does not establish
completion. Mark done only when every criterion is met and all required child
work is done. If scope changes, update the outcome and checks explicitly. Split
independent outcomes into children rather than expanding an unrelated memo.
Record a remaining defect as new work, linked from the result.

## Starting shape

```markdown
---
kind: work
description: Persist session drafts between restarts
status: open
---

## Outcome

A session restores its unsent draft after restart.

## Check

- [ ] Restarting restores the draft exactly, including newlines.
- [ ] Submitting clears the saved draft.
```

Statuses, ownership, dependency readiness, and completion rules are authoring
conventions. The memo cartridge stores these fields; it does not yet schedule
work or enforce these lifecycle rules.
