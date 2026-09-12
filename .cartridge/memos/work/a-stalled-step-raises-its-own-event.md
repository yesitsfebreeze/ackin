---
kind: work
description: "A step that runs longer than what it is for emits one stall event naming the run, the step and the elapsed time, and cancels nothing itself"
status: open
level: 11
estimate: 1d
needs:
  - "[[a-listener-subscribes-to-event-types]]"
---

# a-stalled-step-raises-its-own-event

## Outcome

A run that is stuck says so. A step measured against the budget declared for its
type emits, on crossing it, one event naming the run, the step, the tool it is in
and how long it has been there. That is the signal the unblock is acted on: a
watching agent or the user sees a run spending far more than the work is worth and
posts the event that redirects it.

The stall event decides nothing. It does not cut the step, change the phase or end
the run — an automatic cut would be a timeout, and a timeout guesses. Judging
whether twenty minutes is too long for *this* work is the watcher's, and the
signal exists so the watcher has something to judge.

Crossing is reported once per threshold, escalating, not repeated: a long step
produces a few events as it gets worse, never one per tick.

## Check

- [ ] A step held past its budget emits exactly one stall event naming the run,
      the step, the tool and the elapsed time; holding it past a second threshold
      emits exactly one more.
- [ ] A step finishing inside its budget emits none.
- [ ] The stall event alone leaves the phase, the step and the run's outcome
      unchanged.
- [ ] A stall event reaches a subscribed listener and appears on the run's stream
      like any other event.
- [ ] A profile that declares no budget for a type emits no stall events for it.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. The run already measures itself: `elapsed_ms`, `model_turns`,
`reasoning_bytes` and per-tool counts are accumulated and reported as telemetry on
`run_finished` (`builtin/agent/lib.rs`), and the harness already renders a live
telemetry block for the current run below the prompt frame
(`builtin/harness/README.md`). The numbers exist; they are summarised after the
fact and at the end, which is the one moment a stall no longer matters.

The active call is already tracked with its identity and cancellability while it
runs (`lib.rs:113,142`), so the step being timed is already named. What is added is
a deadline per type, an escalating threshold, and one event.
