---
kind: routine
description: The improvement loop — sweep the record and the tree for the next open thread, research it, land the finding as a memo, repeat.
uses:
  - usage: "[[run-usage]]"
    when: [looking for what to work on next, or running a research pass]
---

1. Read

- `memos/SYSTEM.md`, then open-work and method.
- `git status --short` and `git log --oneline -10` — what the tree is
  mid-change on is never the thread this routine picks up.

2. Sweep

Gather the candidate threads, cheapest source first, and stop at the first
source that yields one:

- `rg -l 'status: open' memos/` — work memos nobody ran.
- `rg -n 'A: \?' memos/question/` — questions the drill left open.
- open-work's ranked list, worst-first.
- `gh issue list` when the repo has a remote with issues.
- The tree itself: a `just check` warning, a test marked ignored, a memo
  whose `Check` no longer passes, a claim in a `kind: knowledge` memo the
  code has since falsified.

3. Pick

One thread per pass. Prefer, in order: a falsified claim (the record lies), a
blocked work memo (something is stuck), an open question (something is
undecided), a ranked defect (something is broken). Say which and why in one
line before doing anything else.

4. Research

- Read the code the thread names, end to end, before forming a view.
- `kern query` for what the graph already holds on it — the record answers
  before a grep does. Neither `kern search` nor `kern recall` is a verb; the
  `search` operation is the same read widened across every project on the
  machine and reaches an agent as an MCP tool only, never as a CLI subcommand
  (the-improve-routine-cited-two-verbs-kern-does-not-have).
- Reproduce, measure, or cite. A claim with no evidence is not a finding.

5. Write

The finding lands as exactly one memo in `memos/intake/`, kind chosen
by what it is (SYSTEM's kind table is the authority):

- `research` — what was looked into, findings and what is still unknown.
- `insight` — a connection across memos, argued in the body.
- `knowledge` — a fact about how the code actually works, now verified.
- `work` — actionable, `level: 10`, with a `Do` and a runnable `Check`.
- `question` — the research hit a decision nobody has made; it goes to the
  drill, not into this memo's body.

One claim per memo (SYSTEM's atomicity law), links by leaf name to every
memo the finding touches. The index is generated (the-index-is-derived);
no memo edit writes one.

6. Close and loop

- Author through `memo` writes; report results and warnings. One line: thread picked, memo written, kind.
- Commit what this pass authored, so the finding survives a sibling session:
  `git -C memos add` the memo path written in step 5, then
  `git -C memos commit` immediately after, with a message naming this pass.
  Never `git add -A`, never a path another session left dirty, and no
  generated index beyond what this memo's own regeneration touched
  (git-policy). A `blocked` memo stays blocked once committed —
  committing it is preservation, not approval.
- Go back to step 2. The loop ends when a sweep finds no thread, or when the
  caller stops it — never because a pass produced nothing worth writing;
  that pass says so and picks the next thread.

Nothing here edits code, so a pass needs no lane — `memos/` is one record
shared by the trunk and every lane (lanes-not-a-shared-tree), and a memo
written anywhere is seen everywhere. A pass that wants to change the tree
writes the `work` memo and hands it to [[work]]; a pass that finds a procedure this
repo has now run twice by hand hands it to [[new-routine]]; a pass that finds a
memo already rotten rather than a claim still missing hands it to [[hygiene]].
