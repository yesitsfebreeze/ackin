# Program critique rounds

Requested on 2026-09-12. Review the whole current runtime, tests, and manifest after each meaningful implementation iteration. Each round is evaluated by an independent critique agent. Stop when all three dimensions reach 80/100, or after five rounds. Any dimension below 80 makes that round a failure. Scores are reviewer judgments supported by evidence, not benchmark percentages.

## Fixed rubric

The first critic must define point allocations under these dimensions, then reuse them in later rounds:

- Performance: measured representative runtime behavior, resource management and scaling, and concrete bottlenecks. Record command, workload, build profile, samples, environment, and limitations. Test-suite duration alone is not a runtime benchmark.
- Cleanliness: correctness evidence, understandable architecture, maintainability, dead code, error handling, lint and formatting results. Preserve required functionality.
- Size: unnecessary code and dependencies, duplication, and release binary footprint relative to requirements. Record production source lines separately from tests. Minimize waste rather than arbitrary line counts.

Use the same benchmark workload and point allocations across rounds. Explain any unavoidable measurement or environment change. Record each actionable finding with a code location and the score it affects; fix those findings before a further critique. Note unsupported targets and other validation limitations plainly.

## Round history

| Round | Performance | Cleanliness | Size | Verdict | Evidence |
|---|---:|---:|---:|---|---|
| 1 | 72 | 70 | 87 | FAILURE | [Independent critique](quality-evidence/round-1-critique.md), [baseline measurements](quality-evidence/baseline/README.md) |
| 2 | 84 | 90 | 88 | PASS | [Independent critique](quality-evidence/round-2-critique.md), [measurements](quality-evidence/round-2/results.json), [implementation](quality-evidence/round-2-implementation.md) |
| 3 | 81 | 76 | 85 | FAILURE | [Integration critique](quality-evidence/round-3-critique.md) |
| 4 | 82 | 89 | 86 | PASS (previous layout) | [Integration critique](quality-evidence/round-4-critique.md), [corrections](quality-evidence/round-4-implementation.md) |
| 5 | 82 | 90 | 86 | PASS (final layout) | [Final critique](quality-evidence/round-5-critique.md), [implementation and gates](quality-evidence/round-5-implementation.md) |

Round 1 established the fixed point allocations in its critique. Required fixes: diagnostic array redaction/oversized-record handling; bounded discovery and pre-ready lifecycle; socket subscription cleanup; completed resolver-subtree caching; formatting and strict lint. Round 2 corrected those findings, renamed the runtime to cartridge, restored reload/landscape APIs, and passed 148 tests plus formatting and strict clippy. The final fixture correction changes no measured production code. Runtime ratings now pass. A subsequent integration iteration is reviewed separately within the five-round total; retain the same rubric.

Round 3 reviewed the assembled submodule project. It found incomplete bundle builds/silent missing executables and memory Docker recipes without the sibling SDK mount. These are required round-4 corrections; remote CI and container execution remain explicitly unrun.

Round 4 passed the corrected composition at parent `0f86753`, including actual debug bundles from both invocation contexts and execution after relocation. The user then requested a wrapper containing only submodules, with all orchestration and artifacts owned by `cartridge.ctg`. That new layout is assessed by the final fifth round below; it is not covered by round 4. All five permitted rounds have now been used.

Round 5 FINAL PASS (82 performance / 90 cleanliness / 86 size) reviews parent `4371c66`, runtime `695adbb`, tools `d8abe77`, and memory `ddd4797`. The wrapper contains only 17 submodules and Git metadata; the runtime owns orchestration, docs, target and dist. The complete source helper suite passed 1,726 Rust tests with zero failures and 17 existing ignored, plus UI 34. Final tools tests separately passed 7/7. Three actual bundle contexts, relocation with ledger/SDK smoke, fresh local recursive clone and a new-worktree contract passed. Runtime measurements remain scoped carry-forward evidence. Remote CI/publication, actual Docker execution, complete release bundle size and full-application performance remain unrun. Broader sandbox board work remains open.
