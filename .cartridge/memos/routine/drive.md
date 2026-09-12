---
kind: routine
description: Run a plan on a 20-minute alternation — plan for one interval, execute for the next, forever, until the plan is empty.
uses:
  - usage: "[[run-usage]]"
    when: [running a plan as a long unattended session]
---

1. Read

- SYSTEM (`memos/SYSTEM.md`) first, then [[work]]
  (`memos/system/work.md`).
- The plan memo [[work]] names, and its open work memos.

2. Alternate

- The session runs in 20-minute intervals, alternating between two modes.
  A mode owns its whole interval; nothing carries over mid-interval.
- **Plan interval**: [[drill]] (`memos/system/drill.md`). Answer open `A: ?`
  blocks, split any item that cannot stay at `level: 10`, write the
  children. Produce work memos, never code.
- **Execute interval**: [[work]] (`memos/system/work.md`). Run open work
  memos in the plan's order — lane, pick, do, run the `Check` literally,
  record, land. Produce code and status moves, never new
  plan structure. Each memo's code is born in its own lane and reaches the
  trunk only through `just land` (lanes-not-a-shared-tree).
- The interval boundary is a stopping point, not an interrupt: finish the
  work memo or the question block in hand, then switch.

3. Self-pace

- On a host with same-session completion goals, arm a goal for the whole
  requested backlog before starting. Indexing the backlog is a planning step,
  not completion. A progress report or one landed item does not end the goal.
- Each continuation reads the last checkpoint, does one bounded useful step,
  and records mode, interval start, current item, evidence, blockers, and next
  action in a session checkpoint under `memos/dashboards/`. Re-read the source
  work memo before acting; the checkpoint never replaces its scope or Check.
- Measure the 20-minute intervals by elapsed wall time. At the boundary finish
  the question block or work memo in hand, then switch. Do not sleep or poll
  merely to consume an interval: use remaining time on independent work in the
  current mode. If none exists, checkpoint the exhausted mode and switch early.
- Await a running check only when it blocks the next useful step. Skip work
  requiring a human answer or approval and continue independent ready work;
  recommendations are not answers. Ask the outstanding question when no
  independent work remains.
- Keep the goal armed across turn summaries. Complete it only at the Close
  condition below; report concrete external blockers when every remaining
  path is blocked, following the host's continuation and blocker rules. A
  human pause or stop takes precedence immediately.

4. Scale

- When an interval's work fans out — many files to read, many independent
  items to run — dispatch it as a workflow (`ultracode`) rather than
  serializing it into the next interval. One agent per work memo, each in
  its own lane: the `WorktreeCreate` hook gives an isolated agent the same
  `just lane` a hand would, and its first build is seconds because kache
  already holds every crate a sibling lane compiled.

5. Close

- The loop ends when no memo in the plan is `open`. Report managed memo-write
  results and warnings, run `just check` and `just test`, and give one line per
  memo — done, blocked, or split.
