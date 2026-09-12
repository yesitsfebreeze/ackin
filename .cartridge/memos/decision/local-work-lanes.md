---
kind: decision
description: "Work lanes integrate into the current snapshot branch and leave the tracked record to the coordinator"
status: accepted
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["Running work or landing a task in this checkout"]
---

# local-work-lanes

## Decision

The user requested finishing every task after the missing lane recipes were reported. Bootstrap local lane recipes in an isolated Git worktree, then use them for task dispatch. The integration branch is `snapshot/plugin-workspace`, the branch checked out at `/Users/feb/dev/sys`; `main` lacks the plugin workspace and is not reset or overwritten. No commands access the original kern checkout.

Code workers own one branch and assigned file footprint. They report acceptance evidence and never land. The coordinator serially reviews, integrates, and records completion through kern memo writes. This checkout tracks `.zirkle/memos/` in the code repository, so lanes keep ordinary tracked copies; workers do not write their copies or share a symlink. The coordinator owns record changes in the trunk and commits only those paths before integration.

## Why

The installed work routine assumes lane recipes, a separate record repository, and a main branch containing the current code. This checkout has none of those conditions. The local workflow preserves the existing snapshot, unrelated untracked `.agents/`, and another session's worktree.

## Consequences

The checkpoint lives at `documentation/work-checkpoint.md` under the declared documentation kind. Missing terminal recipes mean visible editor integration is unavailable; source and Git diff inspection remain available. Close only completed lanes whose commits reached the integration branch; preserve unfinished work and meaningful ignored files.
