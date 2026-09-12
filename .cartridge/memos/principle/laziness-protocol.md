---
kind: principle
description: Apply when refactoring, sizing a diff, or tempted to add abstractions, layers, or signal threading. Bias toward deletion and the smallest change that solves the problem.
group: core
---

# Laziness protocol

Aim for the most result with the least code and complexity.

**Why:** Every layer, flag and pass-through is a coordination cost paid by
every later reader and every later change. Code that is exhausting to maintain
is a bad solution however elegant it looked when written.

**The pattern:**

- **Prefer deletion.** Asked to refactor or improve, look for removals before
  additions.
- **Keep the call hierarchy flat.** A rich interface that hides substantial
  work is not a deep call chain. If answering a question means tracing more
  than three files or layers, flatten it.
- **Consolidate decisions.** Do not repeat one choice in several places. Put
  it behind one source of truth and pass the result as a simple flag.
- **Minimize the diff.** The smallest change that solves the problem. Fewer
  lines beat elegant boilerplate.
- **Question the threading.** A task that asks you to pass a new signal
  through types, schemas and pipelines is a task to stop and find a more
  direct path for.
- **Sweat the small leaks.** Tiny pass-throughs, representation leaks and
  duplicated choices compound into permanent coordination costs.

**The test:** would a human developer find this code exhausting to maintain?

Sequencing is [[subtract-before-you-add]]; the reader's side of the same cost
is [[minimize-reader-load]]. This record's own version of the rule is
[[delete-superseded-work]]: the old design goes in the same change.
