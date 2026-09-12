---
kind: routine
description: Design before implementing — ground in the existing system, sketch types and boundaries as competing candidates, synthesize one, fill it in, and scrap it when implementation proves it wrong.
uses:
  - usage: "[[run-usage]]"
    when: [starting non-trivial work where jumping to code would lock in the wrong shape, designing a module boundary or a set of types]
---

Sketch types, signatures and module boundaries with unimplemented bodies and
pseudocode. Synthesize across several perspectives, then fill in code against
the chosen sketch. If implementation proves the sketch wrong, throw it out and
redesign.

1. Ground the problem

Build a real mental model of every system the new code touches. Run
[[explain-how]] over the relevant subsystems. Naming a file is not grounding —
produce the traced model that routine prescribes. If the design redefines
ownership or layering, also run [[explain-why]] on the existing shape so the
rationale becomes a constraint and not a guess.

Skip this step only when the work is genuinely greenfield with no surrounding
system to integrate.

2. Sketch

Run [[arena]] with the design-sketch task and the step 1 grounding. Each
candidate produces types, signatures and a short rationale naming what it
considered and rejected.

Design it twice. Require at least two structurally distinct candidates before
synthesis, even when the first looks sufficient — that is
[[exhaust-the-design-space]] made concrete. Whole-shape alternatives, not
point fixes inside one shape.

Screen every candidate for shallow modules, information leakage, temporal
decomposition and pass-through methods. Reject or revise before synthesis.

Compare survivors on interface depth. Prefer the design that hides more
complexity behind a smaller, simpler public surface. A rich interface keeps
call chains short by concentrating capability instead of scattering it, per
[[minimize-reader-load]].

3. Agree (opt-in)

Default: proceed to implementation with the synthesized design, per
[[never-block-on-the-human]]. Stop for sign-off only when the caller asked for
a checkpoint.

Either way the synthesis can land as its own commit — the scaffold-first mode
of [[foundational-thinking]]. Planned, scoped breakage during fill-in is fine,
per [[outcome-oriented-execution]]. For adversarial pressure on the design
before implementing, run [[interrogate]] on the sketch.

If the human pushes back on the shape, in a checkpoint or after the fact, that
is step 1 evidence. Re-ground and re-run step 2 before writing more code.

4. Implement against the sketch

Replace unimplemented bodies with code, pseudocode with logic. The synthesized
sketch is the contract.

Deviations are signal worth surfacing, not friction to absorb silently. If a
function needs a parameter the sketch did not anticipate, ask whether the
sketch was wrong, the requirement was missed, or the implementation is
overreaching.

5. Scrap when the architecture is wrong

If implementation keeps producing friction the sketch cannot absorb, throw the
sketch out. Do not bolt fixes onto a wrong design, per
[[redesign-from-first-principles]] and [[fix-root-causes]].

The signal is a pattern, not single instances. The tells:

- The same shape of workaround appearing repeatedly across unrelated code.
- Several unrelated edge cases that all need special-case branches.
- Types that need escape hatches to compile.
- The "we need a lock" reflex when the sketch said the state was not shared.
- Callers having to know the abstraction's internal rules to use it.
- Two or more independent step 4 deviations of the same shape.

Use judgment. A few edge cases do not condemn an architecture. Complexity in
the data is not complexity in the design.

When you scrap: rerun [[explain-how]] over what has been built, redesign as if
the new constraints had been day-one assumptions, subtract before adding per
[[subtract-before-you-add]] so the new sketch starts smaller than the old one,
and return to step 2.

Check: the caller's usage is written first and the type sketch derived from
it. One file of new types and signatures for a small change; a module map plus
type definitions for larger work. The rationale ships alongside, naming the
base, the synthesis decision, and what was rejected.
