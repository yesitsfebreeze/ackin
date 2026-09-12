---
kind: note
description: "The Cordis paper: A Programming Paradigm for Spatiotemporal Composability (Shi, Zhang, Cui; arXiv 2608.25512, 2026-08-26)"
uses:
  - usage: "[[read-usage]]"
    when: ["citing the Cordis paper or looking up its formal model"]
    tags: [reference]
---

# cordis-paper

arXiv <https://arxiv.org/abs/2608.25512> (PDF at `/pdf/2608.25512`; the HTML
view returned 404 on 2026-09-09). Source and BibTeX:
<https://github.com/cordiverse/paper>, a preprint under active revision —
cite the arXiv version.

Worth following for: §3 revertible effects and reactive coeffects, §4 the
calculus and its metatheory (Theorem 80, confluence), §5 the implementation
map (Table 2 pairs every formal symbol with a runtime name; Algorithms 1–10
are the core, loader and HMR), §6 the honest discussion of limits.

Findings drawn from it: [[cordis-revertible-effects]],
[[cordis-reactive-coeffects]], [[cordis-context-mediation]],
[[cordis-loader-and-hmr]], [[cordis-guarantees-and-limits]].
