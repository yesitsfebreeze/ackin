---
kind: decision
description: "Code changes live in lane worktrees; the shared checkout carries only record edits, committed within minutes"
status: accepted
date: "2026-09-12"
uses:
  - usage: "[[read-usage]]"
    when: ["Writing code or record while another session may be working in the same checkout"]
---

# the-trunk-checkout-is-a-landing-pad

## Decision

`/Users/feb/dev/sys` is where lanes land, not where work is done. Every code
change belongs in its own worktree from `just lane <name>`. Record edits on the
shared checkout are fine — the record is one repository shared by the trunk and
every lane, so a lane would isolate nothing — but they are staged by path and
committed within minutes, never left sitting. A worker stages only the paths it
owns; `git add -A` on the shared checkout is nobody's move.

Agreed on 2026-09-12 by the three sessions then working this tree (sys-6d,
sys-fb, sys-de) after it went wrong twice in one hour.

## Why

`just land` and `just lane-rm` both refuse while the trunk has tracked changes:
`tracked changes in /Users/feb/dev/sys; commit them in their owning lane first`.
One person's uncommitted file therefore blocks every other session's landing,
and the block is silent until someone tries to land. Two landings were parked
this way in one pass.

The second failure is worse than the first. A `git add -A` on the shared
checkout swept another session's in-flight memo edit into an unrelated commit.
And uncommitted code on the trunk is indistinguishable from abandoned dirt: this
session read a live author's files as leftovers and moved them to a branch,
because the only evidence available was an mtime. A file written sixty seconds
ago and a file written an hour ago look identical in `git status`.

## Consequences

Before treating anything on the shared checkout as abandoned, check the mtime
against the current clock and ask the machine who is live (`ListAgents` names
the busy sessions). Ownership is established by asking, not by inference.

A landing wave starts by confirming the trunk is clean. When it is not, the
owner lanes their work first; parking someone else's files is a last resort,
and only after the owner has been identified and has agreed.
