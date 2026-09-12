---
kind: type
type: principle
description: A standing rule about how work is done, with the situation that triggers it
uses:
  - usage: "[[read-usage]]"
    when: [applying a standing rule to a change, judging whether a design or diff holds up]
  - usage: "[[compose-usage]]"
    when: [writing a principle memo, naming a recurring correction as a rule]
---

# Principle

A `kind: principle` memo states one standing rule about how work is done and
names the situation that triggers it. It governs judgment, not a procedure: a
[[routine]] says what to run, a principle says what the result must be true of.
A principle is not a [[decision]] — a decision settles one choice in this
project, a principle holds across projects and outlives any one design.

## Fields

Every instance has `kind: principle`, a nonempty `description`, and `group`
(one of `core`, `architecture`, `verification`, `delegation`, `meta`). The
`description` opens with the trigger — the situation in which a reader should
apply the rule — because [[principles]] composes descriptions into the system
prompt as the index, and the trigger is what makes one principle findable
among many.

## Body

- The rule in one or two sentences, first, before any elaboration.
- `**Why:**` the cost the rule avoids. A rule without a cost is a preference.
- The pattern: the concrete moves that apply it.
- The test: the question a reader asks to know whether they violated it.

Link neighbouring principles with `[[name]]` and say how they differ. A
principle that cannot be distinguished from its neighbour should be merged
into it.

## Writing one

Write a principle when the same correction has been given twice and the fix is
judgment rather than a mechanism. When the fix is a mechanism, build the
mechanism instead — that is [[encode-lessons-in-structure]], and it applies to
this record too. Keep the body short enough to read in full at the moment of
use; a principle nobody reads governs nothing.

## Starting shape

```markdown
---
kind: principle
description: Apply when tempted to add a layer. Prefer deletion and the smallest change that solves the problem.
group: core
---

# Laziness protocol

Aim for the most result with the least code.

**Why:** ...
```
