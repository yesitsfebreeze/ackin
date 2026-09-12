---
kind: routine
description: Keep a reviewable decision trail for long or unattended work — one tab-separated row per decision with what, why, the evidence and the result, audited against the transcript before handing back.
uses:
  - usage: "[[run-usage]]"
    when: [running autonomous or multi-phase work, working on something the human reviews after stepping away, needing an auditable trail a reviewer can replay]
---

Keep one canonical log. Other routines route their trail here instead of
inventing a format; reference this by name and let it own the columns.

1. The format

One tab-separated file, one row per decision. Cells stay single-line. Evidence
is a pointer, not prose.

- **ts.** ISO 8601 timestamp.
- **phase.** The phase or workstream.
- **decision.** What was chosen or done, one line.
- **why.** The reason in plain words. If a principle drove it, say it plainly,
  not as a jargon tag.
- **evidence.** A link or path that proves it: a commit, a `file:line`, an
  artifact or trace path. Never a paragraph.
- **result.** The outcome or predicate state: `tests green`, `reverted`,
  `INCONCLUSIVE`, `open`.

An illustration, not rows to copy:

```
ts	phase	decision	why	evidence	result
2026-09-12T09:02:00Z	frame	counted the work first, 100 items, 75 hours	wanted the size before a long run	commit 3a9f1c2	found 5 things to sort out first
2026-09-12T11:15:00Z	widget	moved the styles over without changing how it looks	keep the change small and the result identical	commit 7c21e0a, pixel-diff 0	identical, tests pass
2026-09-12T12:30:00Z	widget	threw out a delegate's work, its screenshots were blank	checked the real files instead of its summary	worktree reset	reverted, tightened the brief
```

2. Logging a row

Write each entry the way you would tell a colleague what you did. Plain words,
concrete actions — [[unslop]] applies to log text too.

Log decision points and checkpoints, not every action: a fork chosen, a unit
completed with its verification result, a pivot or revert with its trigger, a
blocker surfaced, a gate fixed. One row per iteration for a loop. Skip the
trivial and self-evident.

Stamp the timestamp mechanically and strip stray tabs and newlines from cells.
A cell beginning with `=`, `+`, `-` or `@` gets a leading quote so a
spreadsheet does not evaluate it.

3. Where it lives

By default the log is a working artifact, not committed: `decisions.tsv` in
the work directory, or one file per effort when several run at once, kept out
of git.

Commit it only when the work is ambitious enough that a reviewer needs the
trail to trust the result.

4. The rules

- One row is one decision or checkpoint.
- Append-only. A wrong call gets a new row that supersedes it. Never edit or
  delete history.
- Prefer evidence produced by committed scripts over hand-made one-offs, per
  [[encode-lessons-in-structure]].

5. Audit the log against the transcript

At the end of the run, before handing back, check the log told the truth. Walk
it against what actually happened:

- Every row maps to a real action. Cut invented or aspirational entries.
- Each row's evidence resolves and shows what the row claims.
- A fork, pivot or abandoned approach that shaped the work and is not logged
  is a gap. Add it.
- Drop padding.

Fix the log, not the story. If the work diverged from what a row claims, the
row is wrong.

6. Cross-review the trail

Before handing back, dispatch a reviewer on a different model family from the
one that did the work. Self-review is not a substitute, per
[[prove-it-works]]. It reads the trail and the run, then flags what the human
should pay attention to — not a redo of the work, a scan for what is
suboptimal or risky:

- Decisions logged with weak or absent evidence.
- Verification steps skipped or claimed without proof.
- Choices that look risky in hindsight — premature, scope-creeping, papering
  over a symptom.
- Gaps a casual skim would miss.

Check: every reply for a run that produced a trail ends with an Attention
section. Lead with the reviewer's model on its own line, then list each flag
pointing at specific rows or moments. "No flags" is a valid value; the model
name alone is not.

Failure: the audit finds rows that never happened. Cut them, say in the reply
that the log was corrected, and treat it as evidence the run was narrating
rather than recording.
