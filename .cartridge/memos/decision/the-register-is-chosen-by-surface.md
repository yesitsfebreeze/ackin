---
kind: decision
description: The terse register governs the reply; unslop governs anything written down
status: accepted
date: "2026-09-12"
---

## Choice

Prose in this workspace takes one of two registers, selected by the surface it
lands on and by nothing else.

The **reply** — what the user reads in the terminal, this turn — is terse:
articles, filler, pleasantries and hedging dropped, fragments allowed, the
answer first. [[dispatcher]] carries the discipline and its limits.

**Anything written down** — a memo, a commit message, a document, a README, a
PR or issue body, a message to a third party, a persona body — is whole
sentences with their articles and verbs, through [[unslop]]. [[writer]] carries
that one.

[[register]] is the enforcing system memo. Compression never applies to a
negation, a number, a unit, a quoted error, a command, or to a security
warning, an irreversible-action confirmation or a sequence whose order a
fragment would scramble.

## Why

Both rulesets arrived as external harness plugins and both already drew this
line in their own boundary sections, but neither could see the other, so the
contradiction was live: [[unslop]] rule 33 forbids dropped articles, verbless
fragments and abbreviations, which is precisely what the terse register is made
of. Written down, the contradiction had to be resolved rather than carried.

Surface is the right axis because the two costs are different. A reply is read
once, now, by someone waiting — attention is the scarce thing and compression
buys it. A memo is read later by someone who was not in this conversation and
cannot ask a follow-up — there, a dropped article is a decode step and an
omitted verb is an ambiguity that nobody is around to resolve.

The alternatives both lost something real. One register everywhere means either
memos that read as telegrams or replies that spend the reader's attention on
grammar the reader did not need. Keeping the terse register but deleting
[[unslop]] rule 33 would have removed the only rule that protects the record
itself from the compression.

## Consequences

- [[unslop]] rule 33 carries a scope clause naming the reply as out of scope,
  so the routine no longer contradicts the register it does not govern.
- [[register]] composes into the system prompt at order 14, ahead of
  [[principles]]. Setting `enabled: false` on it turns the split off; there is
  no other switch and no hook.
- The behaviour previously supplied by the caveman and ponytail harness
  plugins now lives in this record. Those plugins are removed in the same
  change, per the no-legacy-retention law: git is the archive.
- [[dispatcher]] is a new persona and [[engineer]] is a workspace memo
  shadowing the memory cartridge's, with the laziness ladder folded in and its
  merge recorded in the body.
