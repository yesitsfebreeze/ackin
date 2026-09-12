---
kind: principle
description: Apply when reviewing or shaping code that is hard to trace. Count the layers between question and answer and the hidden state the reader must hold; collapse one-caller wrappers and shrink mutable scope.
group: core
---

# Minimize reader load

Maintainability is the work a reader must do to understand code. Track two
axes:

1. **Layers to trace.** How many indirections sit between the question and the
   answer.
2. **State to hold.** How much hidden or mutable context the reader must keep
   in their head.

**Why:** Code is read far more than it is written. Line count, cyclomatic
complexity and clean architecture are proxies; reader load is the thing that
matters. The two axes are independent — a flat file with fifty globals is as
hard to reason about as a six-layer adapter stack. Guard both. This is the
human analogue of [[guard-the-context-window]]; working memory is finite for
readers too.

**The pattern:**

- **Collapse layers that cost more than they save:** wrappers with one caller,
  adapters with no second implementation, speculative indirection never
  needed. Inline them.
- **Make adjacent layers change the abstraction.** A layer repeating the same
  methods and arguments adds reader load without compression.
- **Demand interface compression.** A broad interface hiding little complexity
  makes readers learn both the surface and the implementation. Prefer
  boundaries that hide meaningful decisions.
- **Shrink state scope:** pure functions over mutations, locals over fields,
  fields over module state, module state over globals. Derive instead of sync.
- **Name the invariant at the boundary,** not in every consumer, so the reader
  learns it once.
- Before adding a layer or a piece of state, ask whether it reduces reader
  load somewhere else by at least as much.

**The test:** can a new reader answer "where does X come from?" and "what can
change X?" in under thirty seconds? If not, cut layers or cut state.
