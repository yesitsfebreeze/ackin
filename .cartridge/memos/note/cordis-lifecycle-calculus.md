---
kind: note
description: "The Cordis calculus drives every fiber by comparing a committed view (which fiber provided each declared key) against a target view; nine rules, a four-state machine, and one guard that holds a provider's inverse until its dependents are gone"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking how a Cordis component actually moves between inactive, loading, active, and unloading"]
    tags: [research]
---

# cordis-lifecycle-calculus

Paper §4. A component is a triple: `d` the keys it declares, `p` the keys it
may provide, `e` its effect iterator witnessed at `d ∪ p`. A fiber is one
instantiation with a parent, its own binding table, a retirement flag, and a
state: Inactive, Reloading(iterator, accumulator, view), Active(accumulator,
view), Unloading(accumulator, view). The coeffect context is not stored; it is
the union of the tables of Active fibers only, so a fiber stops providing the
moment it leaves Active, before any inverse runs.

The target view of a fiber maps each declared key to the fiber currently
providing it, or is ⊥ when retired or unsatisfied. The committed view is the
resolution the fiber activated against. Every lifecycle rule fires on the two
agreeing or differing: L-Begin (Inactive, target ≠ ⊥), L-Iter/L-Finish (one
iteration while target still equals the view), L-Divert (view differs during
loading: go to Unloading with the inverses so far), L-Leave (Active, view
differs: go to Unloading, act later), L-Unload (run the accumulator, but only
when no installed fiber's committed view still names this one — the guard).
Orchestration is only O-Insert, O-Retire (sets a flag, unconditional),
O-Remove (only when Inactive, table empty, no children). A child a fiber
instantiates is itself an effect whose inverse is O-Retire, so parent teardown
cascades.

The guard is what gives dependents time to run teardown code that still reads
the withdrawn key (Theorem 70), and it cannot deadlock because an Unloading
provider already left the context, so every dependent's own target turned
(Theorem 73, assuming the provider relation is acyclic and instantiation
depth is bounded). Recovery exactness (Theorem 68) needs pairwise
independence: distinct keys commute outright, one key's operations must
commute by the provider's witness, and entangled pairs are ordered by the
rules themselves.

Read from the full PDF ([[cordis-paper]]); the implementation's rendering of
these rules is Algorithm 5, covered in [[cordis-reactive-coeffects]] and
[[cordis-teardown-ordering-gap]]. What would change this: a revision that
carries realms into the calculus (§4.4 sketches it as a larger key set).
