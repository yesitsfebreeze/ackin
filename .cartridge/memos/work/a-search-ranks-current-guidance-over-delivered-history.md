---
kind: work
description: "The landscape's top five answer with the routine that applies, not with work memos whose names happen to carry the query's words"
status: open
level: 10
estimate: 3h
uses:
  - usage: "[[read-usage]]"
    when: ["a search returned the wrong thing, or tuning how the landscape ranks"]
---

# a-search-ranks-current-guidance-over-delivered-history

## Outcome

Asking the landscape how to do something returns the thing to do it with. A
routine that declares the situation outranks a delivered work memo whose name
happens to share words with the query, and a `done` or `cancelled` memo never
outranks current guidance on the same subject.

Scope is ranking. What is indexed is [[one-search-covers-the-record-and-memory]];
this is what order it comes back in.

## Check

- [ ] "run only the gates the change touches" returns `routine/repository-checks`
      in the top five; today it returns four delivered work memos and a fifth
      that matched on the word "only".
- [ ] "which gates do I run before committing" still returns that routine first —
      the query that already works does not regress.
- [ ] A memo with `status: done` ranks below a memo of the same match strength
      without one, and the hit says so, so a caller can see why it was demoted.
- [ ] A name matching two query words does not outrank a declared `when:` phrase
      that matches the situation; the ranker's own tests carry both cases.
- [ ] `cargo test -p landscape` passes.

## Approach

Measured 2026-09-12 against the live record, 463 memo nodes, mirroring
`landscape::search` exactly (same tokens, same STOP list, same field weights
name 3.0 / when 3.0 / description 1.0):

```
Q: which gates do I run before committing
    9.0 [routine] routine/repository-checks.md — when: before,committing; description: before,gates,run
    8.0 [work]    work/gates-run-in-a-lane.md — name: gates,run
    8.0 [work]    work/lane-rm-refuses-after-the-gates-run.md — name: gates,run

Q: run only the gates the change touches
    8.0 [work]    work/gates-run-in-a-lane.md — name: gates,run
    8.0 [work]    work/lane-rm-refuses-after-the-gates-run.md — name: gates,run
    7.0 [work]    work/the-run-is-a-stream-of-typed-events.md — name: run
    6.0 [work]    work/fresh-checkouts-can-run-the-gates.md — name: gates,run
    6.0 [work]    work/lua-only-core-boundary.md — name: only; when: only
```

The same question phrased two ways answers correctly once and not at all the
second time. Three defects, each visible in that output:

1. Delivered history ranks as current. Those four work memos are `done`; the
   record already carries `status`, and the resolver already refuses to read a
   done item as current prose — the ranker does not use it at all.
2. A name is scored as a bag of words at the same weight as a declared
   situation, so two coincidental words in a hyphenated name beat a `when:`
   phrase written for exactly this question.
3. The STOP list is short enough that "only" scores. It holds fifteen words
   against a record whose vocabulary is this dense.

The standing multiplier from the observation journal is the right lever for a
fourth effect and already exists — used guidance rising — but it cannot fix
ordering that is wrong before any observation lands.

Worth building on the measurement rather than on taste: the script that produced
the table above is twenty lines and mirrors the Rust, so a change to the weights
can be checked against real queries before it ships.
