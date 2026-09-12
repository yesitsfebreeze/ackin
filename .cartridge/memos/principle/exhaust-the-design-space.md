---
kind: principle
description: Apply to a novel interaction or architectural decision with no precedent in the codebase. Build two or three competing prototypes and compare side by side before committing.
group: core
---

# Exhaust the design space

When a novel interaction or architectural decision has no established
precedent, explore several concrete alternatives before implementation.
Building the wrong thing costs more than exploring three options.

**The rule.** When the right answer is not obvious, build two or three
competing prototypes or sketches. Compare them side by side. Only then commit.
"Design it twice" is this rule by another name. A second flavour of the first
shape does not count — the candidates must be structurally distinct.

**When it applies:**

- Novel interactions with no prior art in the codebase.
- Architectural choices with multiple viable approaches.
- Design decisions where the outcome depends on feel, not logic.

**When it does not:**

- Mechanical implementation where the pattern is established.
- Bug fixes or refactors with a clear target state.
- Changes where constraints dictate a single viable approach.

[[arena]] is the routine that runs this.
