---
kind: work
description: "The run journal's kinds become a declared vocabulary, each type stating whether it reaches the model and in what frame"
status: open
level: 11
estimate: 1d
---

# an-event-declares-its-type

## Outcome

The strings the run writes into its journal become a declared vocabulary. A type
declares its name and what it means for the model: `message` projects as the
conversation it already is; a type that carries no conversation — an
explanation, a memory, a notice from another agent — either projects in a
labelled frame of its own or does not project at all, and says which. The
projector asks the type. It does not carry a branch per kind, and adding a type
is a declaration rather than an edit to the harness.

A journal holding a type this build does not know still replays: the unknown type
is kept, skipped, and named as skipped where the run's context is inspected, so an
old session is never silently thinner than it was.

The vocabulary is the run's, beside the writer of the journal, so a cartridge
reading the stream and the harness projecting it agree on one list.

## Check

- [ ] Every kind the run writes today resolves to a declared type, and a fixture
      of existing sessions projects byte-identically before and after the change.
- [ ] A test adds one type with a projecting rule and one with a non-projecting
      rule, and neither requires an edit to the projector: the projecting one
      reaches the assembled request in its declared frame, the other is stored and
      absent from it.
- [ ] A journal line carrying an undeclared type projects the rest of the run
      unchanged, and `/context` names that type as skipped rather than omitting it.
- [ ] A run's inspected context attributes every journal record to a type, so the
      stream can be read by type without parsing messages.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `record(kind, run)` mints `{"v":1,"kind":...,"run":...}`
(`builtin/agent/lib.rs:191`) and every stage adds its fields to it; the projection
keeps `kind == "message"` and drops the rest without comment
(`builtin/harness/working.rs:26`). The types therefore already exist as an
undeclared list split across two crates. This memo names them in one place and
moves the keep/drop decision onto the name.

Compatibility is the whole risk: `v:1` records are on disk, and the turn ring
still reads them (`builtin/harness/README.md`, "The turn ring"). Nothing about an
existing kind's meaning may change here — this is a declaration over what is
already written, and the behavioural changes belong to the memos that need them.
