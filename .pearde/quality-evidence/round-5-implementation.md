# Round 5 implementation — source-owned collection

The user requested that `/Users/feb/dev/cartridge` contain only submodule directories and Git metadata. The final correction moves orchestration, documentation, catalog, validation logs, target and distribution artifacts into the source runtime `cartridge.ctg`. The wrapper contains 17 submodules plus `.git` and `.gitmodules`.

Runtime remains a standalone Cargo workspace. Thirteen sibling port packages explicitly select the source-owned virtual workspace at `cartridge.ctg/workspace/Cargo.toml`; memory remains standalone. Runtime production source is unchanged from the prior passed runtime candidate. Cargo development/test profiles disable debug symbols and incremental compilation to share bounded build output.

Tools correction commit `a06cc2feedb0b906ac54a36bc7204518cc3bf114` changes only `tools.ctg/service.rs`: resolve a wrapper without Cargo.toml through its runtime child, recognize the grouped workspace's owning runtime, include runtime/grouped/memory workspaces, and use runtime target/dist from each invocation context. Earlier layout support and missing-binary refusal are retained.

Focused tools validation: seven tests pass; the new real Cargo-metadata fixture covers wrapper, runtime, grouped workspace and sibling package entry points for Debug and Release plans. A separate rerun with CARGO_TARGET_DIR removed proves default target ownership belongs to the runtime source, not the grouped workspace. Package formatting and strict all-target tools Clippy both pass (exit 0). Actual final integration evidence follows below.

The independent fifth critique is reserved until the parent completes actual final-layout build/test/bundle/relocation/clone validation. Round 4 remains a historical PASS for the previous parent-owned layout; it does not assess this new arrangement.

Historical round-3/round-4 paths named parent `validation/` or `dist/` now reside under `cartridge.ctg/validation/` and `cartridge.ctg/dist/`; their original commands and reports remain historical records.

Remote CI/publication, actual Docker execution and complete application Release/performance measurements remain unrun. The prior standalone runtime benchmark may be carried forward only with that scope. Sandbox macOS policy, Linux policy and launch-authority board work remains open.

## Final composed gates

Candidate at review dispatch: wrapper `7af9572`, runtime `4acb234`, tools `d8abe77` (source correction `a06cc2f` plus workspace membership). Final documentation runtime `695adbb` records the completed gates without production changes.

- Source helper build and complete check/test succeeded: **1,726 Rust passed, zero failed, 17 existing ignored; UI 34 passed**. That broad run contained six tools tests; the final tools package was separately run with all seven passing. Do not conflate the separate run with a new full-suite total.
- Actual Debug bundles succeeded from runtime source, wrapper without Cargo.toml, and the grouped workspace. All resolve to source `dist/cartridge` and use one target/profile for runtime, grouped ports and memory.
- Final bundle smoke verified **15 manifests and 14 executables** (runtime plus 13 compiled cartridges), hash-matching build outputs. The bundle was relocated outside the workspace; ledger and an actual SDK `sessions.list` request returning `[]` succeeded.
- Fresh recursive local clone succeeded with **17 submodules**, only submodules and Git metadata at wrapper root, clean status, and correct source/grouped/memory Cargo metadata (1/13/28 packages). Local URL overrides were used; this is not remote publication evidence.

Logs are in source `validation/round-5-{build,check,tests,contract,ledger,tools-build,bundle-source,bundle-wrapper,bundle-grouped,recursive-clone}.log`; final smoke and new-worktree gate logs are in the same validation directory. Independent round 5 completed with FINAL PASS: performance82, cleanliness90, size86, using the fixed rubric; see `round-5-critique.md`.

Final new-worktree validation passed: `validation/round-5-lane-contract.log` reports one contract; `validation/round-5-lane-links.log` reports 15 links and 16 siblings. The disposable worktree and branch were removed, leaving the original 14 linked worktrees intact. No implementation or test work remains before the final independent judgment.

Final reviewed parent `4371c66` pins runtime `695adbb`; the subsequent audit-only commit records reports and does not change the assessed production candidate. All five requested rounds are complete.
