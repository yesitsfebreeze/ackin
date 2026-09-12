# Whole-program critique — round 3 of 5: sibling-repository integration

**FAILURE — Performance 81/100; cleanliness 76/100; size 85/100.**
Every dimension must reach 80. This round scores the shipped integration before
corrections: parent **2fca83a14612c0d0ede266a6452432884d88e9f1**, runtime
**e1ce3a1f73a4f21f7b8a73888fab8a8329ec09cd**, tools **666d75a**, memory **f080ca9**.
The later tools and memory Docker corrections belong to round 4. No code was
edited by this critic. Findings below are direct source analysis; no failing
bundle or Docker build was dynamically executed by the critic.

The scope expands from the standalone runtime to its actual parent workspace,
17 submodules, 15 runnable cartridge links, profiles, launcher, build helpers,
repository tooling and memory's standalone boundary. The review inspected the
port changes and representative integration paths; it did not re-audit every
inherited memory algorithm. Existing dirty runtime material and repaired
worktrees are preserved, rather than counted as new implementation defects.

## Fixed rubric — same allocations as rounds 1 and 2

Scores remain reviewer judgments, not benchmark percentages. The expanded
integration scope and missing whole-application measurements are explicit.

| Performance allocation | Maximum | Round 2 | Round 3 | Evidence and deductions |
|---|---:|---:|---:|---|
| Representative runtime latency/startup | 30 | 27 | 27 | Unchanged runtime production supports carrying the fixed RPC/startup measurements; no claim about full imported application startup or latency. |
| Resource ownership, bounds and recovery | 30 | 25 | 25 | Runtime ownership fixes remain intact. Separate workspaces share compact development output through the helper. Stream queues/history and steady-state framing remain unbounded. |
| Concurrency and algorithmic scaling | 25 | 20 | 19 | Resolver completed-subtree handling remains intact. Landscape now performs one Git subprocess per unique cartridge directory before filtering/paging; that necessary correctness fix adds unmeasured collection-size cost. |
| Measurement and regression evidence | 15 | 12 | 10 | Strong integration correctness evidence, but no whole-port workload, bundle build timing, sustained session, full-process RSS or performance profile. Test duration is not substituted for latency. |
| **Total** | **100** | **84** | **81** | |

| Cleanliness allocation | Maximum | Round 2 | Round 3 | Evidence and deductions |
|---|---:|---:|---:|---|
| Correctness and contract coverage | 35 | 32 | 23 | Local links, cloned metadata, landscape and lane regressions are sound. Shipped bundle command omits required build roots and accepts missing cartridge executables. R3-F1. |
| Architecture and maintainability | 25 | 21 | 21 | Clear sibling ownership, relative links, independent runtime/memory workspaces and import records. Root resolution is repeated between helper, tools and container recipes, which needs focused contract tests. |
| Error handling and lifecycle clarity | 20 | 17 | 17 | Runtime lifecycle evidence carries; helper refuses conflicting binary links; tooling refuses conflicting sibling bindings. No new lifecycle failure established. |
| Formatting, lint and build hygiene | 20 | 20 | 15 | Recorded local gates pass, but memory Docker recipes omit the newly required SDK checkout and cannot resolve their manifest in the container. R3-F2. Remote workflows and cross-target execution remain unrun. |
| **Total** | **100** | **90** | **76** | |

| Size allocation | Maximum | Round 2 | Round 3 | Evidence and deductions |
|---|---:|---:|---:|---|
| Production code proportional to requirements | 40 | 36 | 35 | Imported memory, UI, agent, tools and supporting cartridges serve requested functionality; no arbitrary deletion justified. A larger inherited codebase received integration-focused rather than exhaustive dead-code review. |
| Avoidable duplication/unused structure | 25 | 21 | 21 | Parent contains orchestration and submodule pointers rather than duplicate implementation. Runtime history/evidence accounts for substantial tracked bytes and must be preserved. Build-root selection repeats across entry points. |
| Dependencies and feature scope | 20 | 17 | 17 | Separate locks and optional memory SDK preserve useful boundaries. Lock totals overlap; no claim that their sum is the installed dependency count. Runtime tokio/full observation remains. |
| Release artifact proportionality | 15 | 14 | 12 | Runtime release remains 5.51 MiB. A complete imported application release bundle was neither produced nor measured; debug outputs do not establish its footprint. |
| **Total** | **100** | **88** | **85** | |

## Required corrections

### R3-F1 — High: bundle build plan does not cover the split workspaces

Anchors in reviewed tools commit 666d75a: **tools.ctg/service.rs:191**, **:203**,
**:217**, **:249**; package acceptance at **:101** and **:153–157**.
Affects correctness/contract coverage. This is a source-confirmed integration
regression, not a newly executed failure trace.

`bundle()` obtains Cargo metadata for its current directory and builds only
that workspace. It then separately builds builtin cartridges whose manifests
contain [workspace], which covers memory. In the parent workspace, cartridge.ctg
is explicitly excluded, so a fresh release target has no runtime executable for
the unconditional copy at :101. In the runtime workspace, the first build covers
only the runtime; the sibling Rust cartridge packages are absent from that build.
The package loop at :155 silently skips missing executables, allowing an incomplete
bundle to appear successful when the runtime executable happens to exist.
Cached debug binaries can conceal both cases. The package unit test prepopulates
binaries, so it does not validate the build-root selection.

Build the canonical runtime, parent cartridge workspace and independent memory
workspace into the same target/profile from either entry context. For an actual
Rust/process cartridge, refuse a missing expected executable with an actionable
error; preserve legitimate Lua/UI cartridges with no Rust binary. Test the plan
from both parent and runtime contexts with a clean output directory, and exercise
missing-binary refusal. A command-plan fixture can provide focused evidence without
rebuilding the entire application repeatedly. Do not assume an already-populated
shared target proves the release path.

### R3-F2 — Medium: memory Docker entry points cannot resolve the SDK dependency

Anchors: **memory.ctg/justfile**, recipes `docker` and `docker-test`;
**memory.ctg/Cargo.toml:70**, **:94**.
Affects build hygiene. Source-derived; Docker was not run in this review.

Both recipes mount only memory at /work and execute there. Its new optional path
dependency is ../cartridge.ctg, which is absent in that container layout. Optional
features control compilation, not whether Cargo can read a path dependency's
manifest during resolution. `--no-default-features` alone cannot repair a missing
sibling checkout. CI now fetches the sibling correctly, but the shipped container
entry points were not adapted.

Mount memory and runtime as siblings and select the matching container working
directory, preserving memory-only data scope. Validate the generated command and
both manifest paths. If no Docker daemon is available, record that limitation and
use a narrow path-layout/manifest-resolution check rather than claiming a container
run. Do not widen memory into orchestration or modify user stores.

## Integration evidence accepted

Read parent README.md, PORTING.md, repositories.json, .gitmodules, Cargo.toml,
launcher, scripts/workspace.py; applicable memory.ctg/AGENTS.md; landscape tracked
file projection; tooling composition/bundle/lane logic; memory feature gates and
CI/release workflow changes; runtime production diff and preserved evidence.

- **Landscape tracking corrected:** landscape.ctg/src/lib.rs:393 runs Git in the
  entry's own canonical directory; :426 caches each unique directory within the
  view. This resolves the earlier wrong-owner tracked-file join. The sibling-link
  test at :528 verifies tracked files and excludes untracked content. Existing
  paging/cursor tests pass. No persistent cache with stale repository contents
  was introduced.
- **Worktree linkage corrected:** tools.ctg/service.rs:298 provisions sibling
  bindings beside a runtime lane, preserves matching links and refuses conflicts.
  Focused runtime-lane tests, a real lane contract and nine Python lane tests pass.
  The build helper creates ignored relative executable links into Cargo's actual
  target directory after building. Root resolution and memory manifest
  canonicalization are explicit in scripts/workspace.py.
- **Memory standalone boundary improved:** cartridge feature is default-on,
  adapter binary/test require that feature, and SDK dependency is optional.
  Native --no-default-features check passed with the SDK checkout available.
  Workflows fetch memory and runtime into sibling directories; Windows CLI and
  cross/release builds select no-default-features. This is local source/check
  evidence, not executed remote Windows/Linux CI. The inherited justfile exists;
  an earlier suspicion that it was missing is not a finding.
- **Local checkout portability demonstrated:** validation/recursive-clone.log
  records a fresh local clone with explicit submodule URL overrides, all 17 exact
  submodule commits, 15 links, clean clone status and metadata roots for the parent
  (13 packages), runtime (1), and memory (28). GitHub repositories/publication are
  not implied by successful local overrides.
- **Move and composition verified:** validation/migration.json records 14 preserved
  linked worktrees, repaired under the runtime's new location, plus its main tree.
  Post-move links, launcher identity, ledger and tools contract passed. All 15
  installed manifests and dependency bindings appear in validation/final-ledger.log.

Recorded local full-suite evidence: **1,723 Rust tests passed**, zero failed,
**17 existing memory tests ignored**, plus **34 UI tests**. This includes runtime
148, imported cartridge 237 and memory 1,338. Runtime/cartridge strict Clippy and
format checks, UI TypeScript checking, memory workspace check and memory's separate
`just check` passed. See parent validation/final-{build,check,tests}.log,
validation/layout-regressions.log, tools-lanes.log and final-contract.log. Some
full-suite logs retain the staging path; post-move metadata/link/contract checks
cover the final filesystem location. No broad tests were duplicated by the critic.

The suite does not cover the two broken entry points above. Passing it therefore
does not justify a cleanliness pass for this artifact.

## Measurements and size methodology

**No new runtime benchmark was run.** Runtime src/Cargo.toml/Cargo.lock diff from
round-2 final b1a03ef is empty. Against the original measured f22b0ea snapshot,
only src/sdk.rs differs, in the already-reviewed test-only PID/wait correction.
The round-2 raw benchmark and source hashes remain at
[round-2/results.json](round-2/results.json), with methodology and limits in
[round-2 critique](round-2-critique.md).

Carried scoped measurements: default release, macOS 26.2 arm64, Rust/Cargo 1.98,
Python 3.9.6, persistent Unix socket with 1,143-byte JSON, 320 warmups and 3,200
verified replies per workload. Lua median/p95: **0.024166/0.029458 ms** at one
request, **0.079917/0.139959 ms** at 16. SDK-to-host Lua plus two metadata callbacks:
**0.119333/0.147459 ms** at one, **1.151063/1.342791 ms** at 16. Startup 30 launches,
zero excluded warmups: **8.712/12.257 ms** median/p95, first-launch maximum 257.677 ms.
Daemon-only RSS 11,568 KiB. These are not measurements of an entire UI/agent/memory
session, the new launcher overhead, Git landscape collection or packaging.

Runtime release: **5,782,800 bytes**; runtime active Rust **7,683 production /
6,457 test physical lines**. Runtime dependency counts remain nine direct,
78 resolved normal/build packages and 97 lockfile packages. Parent lock contains
243 package records, memory lock 353, runtime 97; these sets overlap and include
non-production/target-specific records.

Read-only tracked-file inventory before corrections: **1,036 regular files,
16,113,731 bytes** across parent and 17 submodules. This excludes symlink targets,
.git storage, ignored build caches/node_modules/live data and untracked inherited
material. Runtime repository contributes **9,091,539 bytes**, including retained
historical material and review evidence; all other repositories plus parent total
**7,022,192 bytes**. These figures are working-tree content bytes, not disk usage,
clone transfer sizes or compiled bundle footprint.

Active Rust estimate across the collection: **67,920 production / 61,556 test
physical lines**, including comments/blank lines. Count tracked Rust outside
historical runtime copies; runtime counts use src only. Files under tests/test/
fixtures/benches/bench and exact cfg(test) items count as test; sessions main.rs
was manually classified at :858–1449 and :1451–1497 because the runtime-focused
scanner could not parse that file. This is a syntactic estimate, not build-aware
reachability or a finding of unused code. TS/Lua and non-Rust fixtures remain in
tracked bytes, outside Rust LOC. The expanded imported feature set must not be
compared with the old standalone runtime as if every additional line were waste.

## Limits and next round

Fix R3-F1 and R3-F2 before round 4 and preserve focused evidence tied to the new
submodule pointers. Additional broad reruns are needed only for changed behavior
or unresolved failures; do not repeat unchanged runtime microbenchmarks to give
the impression that whole-port performance was measured.

Sandbox **macOS policy, Linux policy and launch-authority board work remain open**.
The command adapter is not full launch confinement. This integration review opens
no new sandbox PRDs and reproduces no new sandbox bypass. Remote CI, release
publication, authenticated provider operations, real Docker execution and complete
application performance/release size remain unvalidated. GitHub URLs are prepared
publication metadata; the tested clone uses local overrides.

Final verdict for the specified pre-correction artifact: **FAILURE, 81 / 76 / 85**.
