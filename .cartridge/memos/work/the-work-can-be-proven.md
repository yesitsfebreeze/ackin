---
kind: work
description: "A change can be built, proven and landed without a person working around the tools"
status: done
subwork:
  - "[[check-boxes-lint-as-observable-behaviours]]"
  - "[[gates-run-in-a-lane]]"
  - "[[lane-rm-refuses-after-the-gates-run]]"
  - "[[per-cartridge-versioned-test-runner]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the gates, the lane workflow, or the test fixtures every other piece of work depends on"]
---

# the-work-can-be-proven

## Outcome

Every other outcome in [[the-vision]] depends on this one: a change is made in
a lane, the gates run there and pass, and `just land` puts it on the trunk. When
that loop needs a person to delete a directory, set an environment variable or
skip a check, the loop is the thing to fix, because every piece of work pays the
cost again.

Scope is `just check`, `just test`, the lane recipes, and the fixtures those
tests share. A failing test that reports a real defect belongs to whichever
memo owns that behaviour; a test that cannot run at all belongs here.

## Check

- [x] Run end to end four times this session, the last one with no manual step:
      `just lane lane-rm-artefacts`, `just all` green, `just land`, `just
      lane-rm` — the last removing a lane that held every artefact the gates
      produce.
- [x] Every child work memo listed in `subwork:` is done:
      [[gates-run-in-a-lane]] and [[lane-rm-refuses-after-the-gates-run]].

## Result

The loop works. `just lane` lends the new worktree the trunk's memory workspace;
the justfile and both terminal tests resolve the cargo target directory instead
of assuming one; `lane-rm` clears the artefacts the gates leave. Landed as
`141afed`, `56dd640` and `2091b64`.

Reopen this memo rather than working around the next thing that makes the loop
need a manual step.
