---
kind: work
description: "A landed lane can be removed after its gates have run"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["just lane-rm refuses with 'untracked or ignored files remain in lane'"]
---

# lane-rm-refuses-after-the-gates-run

## Outcome

`just lane-rm <name>` removes a landed lane whose only leftovers are build
artefacts, without a person deleting directories by hand.

Running the gates in a lane is the normal path, and it always leaves
regenerable artefacts behind: `builtin/ui/node_modules` from the `bun install`
that `just check` itself runs, `core/tests/__pycache__` from the terminal
tests, and since `141afed` the `builtin/memory` symlink that `just lane`
creates. `lane-rm` lists untracked and ignored paths with `git ls-files
--others --directory` and refuses if any remain, so the normal path always ends
in a refusal.

The guard is right to exist — it is what stops an unlanded change being thrown
away with the worktree. It just cannot tell work from build output.

## Check

- [x] Proven on itself: `just lane-rm lane-rm-artefacts` removed the lane this
      work was done in, which held `node_modules`, `__pycache__` and the memory
      symlink after a full `just all`, with no manual deletion.
- [x] `test_removes_a_landed_lane_holding_only_tooling_artefacts` puts a plain
      `mine` file beside the three artefacts, asserts the refusal names it, and
      asserts the file survives.
      `test_refuses_dirty_unmerged_and_ignored_lane` still passes unchanged: an
      ignored `target/cache` a person put there blocks removal and is kept.
- [x] The symlink is removed through `symlink_metadata`, so the link goes and
      the trunk's workspace it points at is never walked into. The trunk's
      `builtin/memory` is intact after four lane removals this session.

## Result

Landed as `2091b64`. `lane-rm` steps over exactly three paths — `builtin/memory`,
any `node_modules`, any `__pycache__` — and removes them, because `git worktree
remove` refuses while any untracked path remains.

Deliberately a named list rather than "everything ignored": that broader rule was
tried first and broke `test_refuses_dirty_unmerged_and_ignored_lane`, which
exists to keep an ignored directory a person put in a lane from being thrown
away with it. `.gitignore` also lost a trailing slash on `/builtin/memory`, which
matched directories only and so missed the symlink a lane holds.
