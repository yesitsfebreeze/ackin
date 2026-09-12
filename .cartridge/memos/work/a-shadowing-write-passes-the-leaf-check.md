---
kind: work
description: A workspace memo that shadows a shipped memo of the same leaf accepts writes; today the write is refused as a duplicate
status: open
level: 10
---

# a-shadowing-write-passes-the-leaf-check

## Outcome

A write to a workspace memo whose leaf is also shipped by a cartridge (the
shadowing case `type/type.md` documents) validates and saves. Today every
write to such a memo fails with "duplicate memo leaf", so the workspace
checkpoint note cannot be updated through the tool at all while the memory
cartridge ships `dashboards/work-checkpoint.md`.

## Check

- [ ] `zirkle run memo` write to `note/work-checkpoint.md` (leaf shared with
      the shipped `@memory/dashboards/work-checkpoint.md`) saves and returns a
      revision.
- [ ] A workspace memo whose leaf collides with a shipped memo still shadows
      it in reads and listings, per `type/type.md`.

## Context

Found 2026-09-12 from the trunk: `builtin/memo/src/record.rs` (which has
uncommitted edits from another session, not to be touched by the fix's lane
until those land) rejects the union of workspace and shipped memos on a leaf
collision before the shadowing rule applies.
