---
kind: principle
description: Apply before writing logic — choosing core types and data structures, sequencing scaffold against feature work, asking what concurrent actors share. Get the data shape right so downstream code becomes obvious.
group: core
---

# Foundational thinking

Structural decisions protect option value. Code-level decisions protect
simplicity.

**Data structures first.** Get the data shape right before writing logic.
Define core types early, trace every access pattern, and choose structures
that match the dominant paths.

At code level, DRY the structure, not every line. Types and data models should
converge. Three similar statements still beat a premature abstraction. Prefer
explicit over clever. Test behaviour and edge cases, not line counts.

**Concurrency corollary.** Before sharing state between actors, ask what
happens if another actor modifies this concurrently. If the answer is not
"nothing", isolate — see [[separate-before-serializing-shared-state]].

**Scaffold first.** If something helps every later phase, do it first. Ask
whether every subsequent phase benefits from this existing. Checks, linting,
test infrastructure and shared types are scaffold. Sequence for option value:
setup before features, tests before fixes. Keep commits small and
single-purpose.

Each increment should land a coherent abstraction or deepen one that exists.
Do not spread a new capability across callers as special-case coordination.

Subtraction comes before scaffolding, per [[subtract-before-you-add]]. This
principle governs the sequence of work; [[experience-first]] governs its
target.
