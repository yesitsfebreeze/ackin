---
kind: work
description: "A priority event cuts the model stream or the cancellable tool the run is waiting on, keeps what it produced, and continues the same run"
status: open
level: 11
estimate: 2d
needs:
  - "[[a-live-run-accepts-events-from-outside]]"
---

# a-priority-event-unblocks-the-step

## Outcome

An event may be more important than the step the run is in. A priority post does
not wait for a boundary: the run abandons the step it is waiting on — the model
stream in flight, or the cancellable tool call it dispatched — records what that
step produced up to the cut and the reason it was cut, appends the event, and
**continues the same run** with it.

This is the unblock, and it is deliberately not a cancel. `cancel` ends a run and
keeps ending it, reaching the terminal phase `cancelled`. An unblock keeps the run
alive and changes what it is doing: an agent spending twenty minutes on something
worth two gets told so and carries on with that knowledge, instead of dying and
starting again from nothing it remembers. A run that was unblocked ends in the
phase its own work reaches.

A step that cannot be cut is not cut. A tool that declares itself uncancellable
runs to its end; the event lands at the next boundary and the journal records that
it waited and for how long, so a step that silently swallows priority is visible
rather than assumed.

## Check

- [ ] A priority post during a model stream ends that stream, keeps the partial
      response as a journal record naming the interruption, carries both the
      partial and the event into the next request, and the run reaches
      `completed` — no `cancelled` phase, no second run.
- [ ] A priority post during a cancellable tool invokes exactly one `cancel` with
      that call's identity, records the interruption as that call's outcome, and
      the run continues; a result arriving after the cut is not appended.
- [ ] A priority post during an uncancellable tool does not cut it; the event lands
      at the next boundary and the journal records the wait and its duration.
- [ ] A priority post while the run awaits an approval is delivered without
      answering that approval, and the approval is still answerable afterwards.
- [ ] `cancel` still ends a run in `cancelled`, and no unblock path can produce
      that phase.
- [ ] An unblocked run's journal replays as one run: the cut step, the event and
      its sender, and the work that followed, in order.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. Both halves of the cut already exist and both are wired to a
terminal outcome. The model stream races a cancel watch and returns
`StreamOutcome::Cancelled` together with `stream.visible()` — the partial is
already in hand and today thrown away (`builtin/agent/stream.rs:116,143`). The
active tool call is already cancelled by its exact identity, once, guarded against
a double send (`builtin/agent/lib.rs:142,454`), and the run already refuses to
append a result that arrives after a cut.

So the work is to split the signal rather than to build a second one: the watch
must carry which of the two meanings it has, `cancel` must keep meaning end, and
the loop must have a resume path out of a cut step. The recovery vocabulary for a
step whose outcome is unknown already exists — `interrupted-outcome-unknown` is
written for a pending call after a restart (`builtin/agent/run_state.rs:497`) —
and a cut step wants the same honesty with a different reason.

The uncancellable case needs no new mechanism: the active call already records
whether it is cancellable (`lib.rs:143`), which is exactly the condition for
deferring to the boundary.
