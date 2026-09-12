---
kind: note
description: "The paper's confluence theorem guarantees only the quiescent state up to an equivalence, assuming author-correct inverses; its evidence is one TypeScript ecosystem, observational, with no benchmark"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["deciding how much of the Cordis paper's promise to trust"]
    tags: [research]
---

# cordis-guarantees-and-limits

Theorem 80 (§4.3.5) says: from the same orchestration steps, any two
schedules reach states equal up to a renaming and the paper's equivalence,
so an application can be reasoned about as if statically assembled. The
paper draws the edges itself. The result speaks of state, not of emissions
along the way (§6.1); failed fibers are excluded since a raise depends on the
schedule (§4.4); inverses and coeffect commutativity are unchecked
obligations on authors (§5.1.1); an async host is inertial and can only land
an iteration, never abort it (§4.4). The case study (§5.3) is Koishi, over
4000 plugins on Cordis v3 while the paper describes v4, and the authors call
it an existence-and-adoption result, not a quantitative one: overhead and
productivity against a baseline are future work.

Reading of the full PDF on 2026-09-09; source-level confirmation in
[[cordis-teardown-ordering-gap]]. What would change this: a benchmark or a
second ecosystem in the next revision of [[cordis-paper]].
