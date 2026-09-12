---
kind: routine
description: Turn a request into planned work — one work memo with a Do that says what, an estimate and a place on the axis under the terminal it serves, so the next pass specs and dispatches it.
uses:
  - usage: "[[run-usage]]"
    when: [a request, a finding or an intake memo needs to become work the plan carries]
---

Pearde's `add` and its analyst's first pass, on the record
(the-pearde-workflow-is-the-work-record). A memo is *in* the plan the
moment it is `kind: work`; it is *planned* only when a chain carries it to
[[the-vision]]. This routine writes both.

1. Read

- memo-writing § "A work item", work-memo, and [[the-vision]]
  (`memos/work/the-vision.md`): its sentence is the destination and its
  `subwork:` names the terminals.
- `kern plan` — or `mcp__kern__plan` — for the board as it stands: the
  `unplanned` section lists every open memo no chain carries to the vision,
  and those are this routine's input as much as a new request is.

2. Name it

- One request is one memo. A request that needs an "and" is two, or a parent
  at level 9 with the parts as its `subwork:` children at level 10.
- The leaf is the claim in kebab-case; `just memo <leaf>` must answer "no
  memo" — a leaf resolves by name alone across the whole record.

3. Write it

```markdown
---
kind: work
level: 10
status: open
estimate: <duration — 30m, 2h, 4h, 1d, 2d; nothing over 2d, that is a split>
description: <the unit in one line>
read_when: <what question sends a reader here>
---

# <leaf>

## Do
<what exists when this is done, why, what must not change — the request as a
contract, never the how: no file names, no verbs of change>
```

- Into `memos/intake/<leaf>.md` (memo-layout); `kern compact` moves it
  to `work/` unchanged. **Write no `## Spec` and no `## Check`**: both are
  the analyst's, written from a probe on the next [[work]] pass — a Check
  written before anything was tried is a check written from the answer. The
  memo lands in the `spec` band, which is where new work belongs.
- `needs:` names what it waits for, as wikilinks: a work memo, or a
  `kind: question` memo when the request already carries a fork — that puts
  it in the `asking` band until the question is answered.

4. Attach it — the step that makes it planned

- Pick the terminal it serves: the open level-9 parent under the vision's
  `subwork:` whose Do the request advances. Append `leaf` to that
  parent's `subwork:` line. A request that serves none of them is a new
  direction: write it at `level: 9`, append it to [[the-vision]]'s
  `subwork:`, and say so in one line — the vision's terminals are the user's
  to keep or strike.
- Maintenance the tree owes itself — a rotten memo, a gate gap, a leftover
  lane — goes under keep-the-tree-and-record-true, the standing terminal
  for it, never under a product terminal it does not advance.
- A parent is never attached under its own child, and a child never
  `needs:` its parent: both are cycles, and the plan refuses the board on one.

5. Check

- Author through `memo` writes and report saved/index status and any warnings.
- `kern plan`: the memo is in a band, the `unplanned` section no longer names
  it, and the hours to the vision moved.
- Commit the memo and the parent's one-line change together in `memos/`.
