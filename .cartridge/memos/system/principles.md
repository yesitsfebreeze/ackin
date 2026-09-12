---
kind: system
description: The standing principles and the situation that triggers each
order: 15
uses:
  - usage: "[[read-usage]]"
    when: [starting any non-trivial change, judging whether a design or diff holds up]
---

Twenty-three standing principles govern how work is done here. This is the
index; each entry names when it applies. Read the leaf memo in full before
applying one, and name the principle and the specific choice it changed when
it shapes a decision. Cite only principles you actually read this session.
[[principle]] declares the kind.

**Core**

- [[laziness-protocol]] — refactoring, sizing a diff, or tempted to add
  abstractions, layers or signal threading. Bias to deletion and the smallest
  change that solves the problem.
- [[foundational-thinking]] — before writing logic: core types and data
  structures, scaffold before feature, what concurrent actors share.
- [[redesign-from-first-principles]] — integrating a new requirement into an
  existing design. Redesign as if it had been foundational from day one.
- [[attack-the-premise]] — two or more fixes sharing one premise have failed
  the same gate. Census which actors hold the imbalance, then question the
  premise instead of writing another fix that assumes it.
- [[subtract-before-you-add]] — sequencing an addition, refactor or rewrite.
  Remove dead weight first, then build on the simpler base.
- [[minimize-reader-load]] — reviewing or shaping code that is hard to trace.
  Count layers and hidden state, collapse one-caller wrappers, shrink mutable
  scope.
- [[outcome-oriented-execution]] — planned rewrites and migrations with
  explicit phase boundaries. Converge on the target architecture, do not
  preserve throwaway compatibility states.
- [[experience-first]] — product, UX or feature-scope tradeoffs. Choose the
  consumer's experience over implementation convenience.
- [[exhaust-the-design-space]] — a novel interaction or architectural decision
  with no precedent. Build two or three competing prototypes and compare
  before committing.
- [[build-the-lever]] — any non-trivial work. Build the tool that does or
  proves it, not by hand. The tool is the artifact a reviewer reruns.

**Architecture**

- [[model-the-domain]] — writing stateful logic, or code that branches a lot
  or repeats a shape assumption across files. Encode the domain in a structure
  instead of scattered conditionals.
- [[boundary-discipline]] — wiring validation, error handling or adapters.
  Guards at system boundaries, trust internal types, keep logic pure.
- [[type-system-discipline]] — designing a type or a signature. Make illegal
  states unrepresentable, brand primitives, parse external data at boundaries.
- [[make-operations-idempotent]] — designing commands, lifecycle steps or
  loops that run amid crashes and retries. Converge to the same end state.
- [[migrate-callers-then-delete-legacy-apis]] — introducing a new internal API
  while old callers exist. Migrate and delete in one wave.
- [[separate-before-serializing-shared-state]] — concurrent actors might write
  the same file, branch, key or object. Eliminate the sharing first.

**Verification**

- [[prove-it-works]] — after a task, before declaring done. Verify against the
  real artifact, not a proxy or "it compiles".
- [[fix-root-causes]] — debugging. Reproduce first, ask why until you reach
  the root, fix it where every caller routes through.
- [[sequence-verifiable-units]] — multi-step work and how you stack commits.
  Small units that each end in a check, verified before the next, ordered so
  the sequence proves itself.
- [[test-behavior-not-implementation]] — writing, changing or keeping a test.
  Call the code the way its users do and assert a literal expected value.

**Delegation**

- [[guard-the-context-window]] — context fills up: large outputs, long files,
  repeated reads, fan-out planning. Route bulk to subagents, keep summaries.
- [[never-block-on-the-human]] — tempted to ask "should I do X?" on reversible
  work. Proceed, present the result, let the human course-correct.

**Meta**

- [[encode-lessons-in-structure]] — you catch yourself writing the same
  instruction a second time. Encode it as a type, test, lint or script instead
  of more text.

The routines these principles name are [[explain-how]], [[explain-why]],
[[architect]], [[arena]], [[swarm]], [[interrogate]], [[blast-radius]],
[[unslop]] and [[show-me-your-work]].
