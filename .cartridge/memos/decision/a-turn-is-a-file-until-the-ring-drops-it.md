---
kind: decision
description: Finished turns keep their session files as a bounded ring; older turns are distilled into the memory bank and deleted
status: accepted
date: "2026-09-12"
decided_by: user
---

## Choice

A turn is the unit of the record. Every dispatch creates its own session, named
by its own request and typed on `agent.kind` (`question`, `build`, `create`, and
`turn` as the default), so the record is a list of turns rather than one growing
conversation.

That list is a ring. The newest `harness.ring_keep` (64) finished turns keep
their session files on disk as hard reference. `harness {op:"ring"}` distills
older ones through the summarizer, ingests the prose into the configured
`memory` bank with the turn's kind, id and name ahead of it, and only then
deletes the file. The agent fires one sweep when a turn reaches its durable
terminal state and never waits for it; a sweep evicts at most `ring_sweep` (4)
turns. A running or approval-waiting turn is never evicted, a turn with no
conversation is dropped without paying for a summary, and an eviction the bank
did not commit deletes nothing and is retried by the next finished turn. A
profile that configures no bank keeps every turn and the sweep is a no-op.

## Why

Three separate facts already held and only the ring was missing: a dispatch
already composes its whole context per request, a session file already survives
the terminal that started it, and the bank already chunks, deduplicates and
keeps provenance. Without a bound the sessions directory grows forever; with one
but no bank, closing the ring would throw work away.

Distillation is the summarizer the harness already uses for working memory, not
a second compactor, so prose about a turn is written once in one voice. The bank
is where precision comes from over time: turns fold into a corpus that
deduplicates, rather than into a directory that only gets bigger.

The sweep is deliberately not on the model round's hot path. Distilling costs a
router request, and a turn that has just finished must not wait for the
bookkeeping of an older one.

## Consequences

- `ring_keep` counts finished turns, not bytes; a turn's own size is still
  bounded by the request budget and working-memory compaction.
- The bank becomes load-bearing for history older than the ring. A profile that
  wants the record kept verbatim must raise `ring_keep` rather than disable the
  bank, which only stops eviction.
- Eviction is the only deleter of session files, and it deletes after a
  committed ingest. A bank that silently accepts and loses a document would lose
  turns; `tool.memory`'s commit check is the guard.
- A turn list that reads and filters by kind is now possible without opening
  any turn; the surface for it is not built. Reattaching a still-open turn after
  the terminal closes needs the transcript buffer replayed instead of the UI's
  in-process event ring.
