---
kind: decision
description: The work-checkpoint journals are refused as a distill candidate while their handoff state is live
status: accepted
date: "2026-09-12"
---

## Choice

`note/work-checkpoint.md` and `note/work-checkpoint-2026-09-12.md` are not
folded, not deleted, and not this pass's candidate. The next [[distill]] pass
should not re-argue the rating; it should re-check the one condition below and
act only once that condition has cleared.

The condition: the second memo is live coordination state for work in flight.
It is the only record that two quarantine stashes in the shared
`builtin/memory` repo (`stash@{0}`, `stash@{1}`, labelled sys-fb) hold the
uncommitted zirkle memory-cartridge port, applied by SHA and never popped, and
it carries the lane tip `b452bb7` rebased onto `c7a0e0d`. Three peer sessions
were coordinating through it during this pass. Deleting or reshaping it would
have destroyed the only pointer to uncommitted work.

## Why

The cluster rated highest on the step 3 probe — 1,788 words per claim against
1,000 for the next densest — so it is where the duplication is, and a later
pass should return to it.

The rating agreed the pair is redundant: `work-checkpoint` scored 4 and
`work-checkpoint-2026-09-12` scored 5, with neither carrying an inbound link
from anywhere in the record. But the returned fold plan carried roughly thirty
evidence bullets, which is substantially the whole of the losing memo. A fold
that carries everything is a rename wearing a merge's clothes: it would have
produced one 3,500-word memo from two, lowering the memo count by one and the
word count by nothing, which fails this routine's own check.

The rating also contradicted itself usefully. It described `work-checkpoint` as
"a coordinator journal whose current ownership is now entirely historical" with
"only a handful of durable findings", then proposed carrying all of it. The
handful is the right unit, not the journal.

Both memos are also the wrong kind for what they hold. [[note]] says a note
"does not assign work or establish a decision", and these are handoff journals
full of ownership, next actions and session state. That is [[work]]'s territory.

The rot is measured, not suspected. `builtin/toolgraph/` and everything under it
is gone from the workspace members list, `builtin/landscape` being the surviving
crate, so `cargo test -p toolgraph` no longer runs. The `.momo/` prefix predates
two renames. Every lane path named in either memo — `.claude/worktrees/program-map`,
`fs-operations`, `session-checkpoint`, `core-boundary` — is gone, and
`.claude/worktrees/` is empty. Against that, all sixty-odd commit SHAs resolve,
and `.cargo/config.toml` still declares one shared `target-dir = "target"`, so
the stale-rlib hazard both memos record is live rather than historical.

## Consequences

- The durable findings are what a later pass should extract into one note:
  the shared-target stale rlib hazard, disk exhaustion presenting as compile
  errors, the `just land` refusal workaround, a crate-scoped Check not being the
  repo gate, and the duplicated `subwork:` key. Each already carries a path or a
  number. The journal around them is what git is the archive for.
- The condition to re-check first: whether the sys-fb stashes have been applied
  or dropped, and whether the lane tips named are landed. Once no uncommitted
  work depends on the pointer, the pair is an ordinary candidate.
- This pass landed `note/shell-*` only. See [[the-register-is-chosen-by-surface]]
  for the unrelated prose decision taken the same day.
