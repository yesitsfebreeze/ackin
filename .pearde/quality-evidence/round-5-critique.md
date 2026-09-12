# Whole-program critique — round 5 of 5: final source-owned layout

**PASS — Performance 82/100; cleanliness 90/100; size 86/100.**
All three dimensions meet the fixed 80 threshold. This is the fifth and final
critique round. Scores are reviewer judgments, not benchmark percentages.

Final reviewed parent **4371c662720d11afb0285e22cf745e0240a1c117**, runtime/source
**695adbb89d5239d48a888f80f95723448365fd45**, tools
**d8abe779b59c745f6e322d5192799e3562ab2ca2**, memory
**ddd479766c8a8a365d8df7abcc35ec8f4c212317**. Runtime 695adbb updates PORTING.md
only after the built/tested source layout at 4acb234. Later audit-only commits
may advance the recorded pointers without changing this implementation assessment.
The critic edited only this report and no implementation or score index.

The completed historical rounds remain: 1 = 72/70/87 FAILURE; 2 = 84/90/88 PASS;
3 = 81/76/85 FAILURE; 4 = 82/89/86 PASS for its previous layout. This round assesses
the user's final requirement that the outer repository contain only submodules
and Git metadata, with all orchestration owned by cartridge.ctg.

## Fixed rubric — unchanged point allocations

| Performance allocation | Maximum | Round 4 | Round 5 | Evidence and deductions |
|---|---:|---:|---:|---|
| Representative runtime latency/startup | 30 | 27 | 27 | Runtime production and release settings are unchanged; fixed round-2 measurements remain scoped evidence. Full agent/UI/memory application behavior is not newly benchmarked. |
| Resource ownership, bounds and recovery | 30 | 25 | 25 | Runtime ownership fixes retained; explicit shared output paths and compact dev/test profiles bound avoidable build-cache growth. Runtime queues/history and steady-state frames remain unbounded. |
| Concurrency and algorithmic scaling | 25 | 19 | 19 | Resolver subtree fix retained. Collection-size cost of per-directory Git landscape calls and full application concurrency remain unprofiled. |
| Measurement and regression evidence | 15 | 11 | 11 | Actual three-context builds, relocated SDK smoke, fresh clone and worktree checks provide integration evidence. No controlled full-port latency, sustained-load or release-build benchmark. |
| **Total** | **100** | **82** | **82** | |

| Cleanliness allocation | Maximum | Round 4 | Round 5 | Evidence and deductions |
|---|---:|---:|---:|---|
| Correctness and contract coverage | 35 | 32 | 32 | Exact root constraint, external workspace membership, bundle completeness/relocation and fresh clone validated; focused final tools tests cover four source contexts. Not every inherited cartridge operation was re-audited or exercised. |
| Architecture and maintainability | 25 | 21 | 22 | Runtime source now owns orchestration, docs, catalog and output; wrapper has one clear linking role. Explicit grouped membership preserves standalone runtime/memory. Related root-selection knowledge still exists in helper and tools. |
| Error handling and lifecycle clarity | 20 | 17 | 17 | Prior runtime cleanup, missing-binary refusal and conflicting-link protection retained. No new lifecycle regression found; broader fault/platform coverage is incomplete. |
| Formatting, lint and build hygiene | 20 | 19 | 19 | Full source helper check/test passes, final tools checks pass and actual clone/worktree/bundle paths work. Docker execution and remote CI remain unavailable/unrun. |
| **Total** | **100** | **89** | **90** | |

| Size allocation | Maximum | Round 4 | Round 5 | Evidence and deductions |
|---|---:|---:|---:|---|
| Production code proportional to requirements | 40 | 35 | 35 | Small layout correction preserves imported functionality; broad inherited memory code remains integration-reviewed rather than exhaustively audited for dead code. |
| Avoidable duplication/unused structure | 25 | 21 | 21 | Files moved into source rather than copied; wrapper retains no orchestration duplicate. Helper/tools layout selection and backward compatibility still carry modest complexity. |
| Dependencies and feature scope | 20 | 17 | 17 | No new dependency sets for this layout; grouped lock moved with its workspace and optional memory SDK boundary retained. Earlier runtime tokio/full observation remains. |
| Release artifact proportionality | 15 | 13 | 13 | Complete debug bundle counted; runtime release remains 5.51 MiB. Full application release footprint is still unmeasured and cannot be inferred from debug bytes. |
| **Total** | **100** | **86** | **86** | |

## Final layout and source review

A direct directory inventory at /Users/feb/dev/cartridge found **exactly 17
submodule directories, .git and .gitmodules**. `git ls-files` contains only the
17 gitlinks and .gitmodules. No launcher, Cargo manifest, scripts, docs, target,
dist or validation directory remains at wrapper level.

cartridge.ctg owns README.md, PORTING.md, repositories.json, launcher, justfile,
scripts/workspace.py, profiles, validation, target and dist. Generated outputs
are ignored. The independent runtime Cargo workspace remains at its repository
root. `cartridge.ctg/workspace/Cargo.toml` contains the 13 external Rust members,
with paths relative to that grouped directory; each member explicitly declares
`package.workspace = "../cartridge.ctg/workspace"`. All 13 declarations were
checked. Runtime and memory remain outside that group, with independent locks.
No duplicate implementation was added to the wrapper.

The source helper resolves sibling repositories through canonical builtin/tools
and uses the canonical sibling memory manifest. Its own runtime metadata chooses
target output; grouped and memory builds receive that same explicit target. This
also allows a provisioned runtime worktree to resolve its original sibling bindings
without assuming the lane's immediate parent is the main wrapper. Launcher paths
are source-relative and independent of the caller's working directory.

Tools source at a06cc2f, retained by d8abe77 membership commit, adds source-owned
layout handling to the existing metadata BuildPlan. Wrapper calls resolve the
runtime before Cargo metadata; grouped-workspace metadata resolves back to its
source composition; source target/dist settings become authoritative. Grouped,
runtime and memory workspaces remain deduplicated and build with explicit common
target/profile. The corrected missing executable check and Lua-only behavior from
round 4 remain intact. No new blocking source defect was found.

The test `source_owned_bundle_unifies_wrapper_runtime_and_grouped_workspace`
(tools.ctg/service.rs:685) constructs real Cargo manifests and external membership,
then checks wrapper, runtime, grouped and sibling-package contexts under Debug and
Release command plans. It validates all three workspaces, source-owned target/dist,
command cwd and profile flags. A separate run without CARGO_TARGET_DIR verifies
that default output is runtime/target; the test does not pass solely because a
shared environment variable masks an incorrect default. Final focused tools suite:
**7 passed**; formatting and strict Clippy pass, as recorded by the implementation
worker. The actual debug executions below independently cover the key paths.

## Executed final integration evidence

All current logs are under **cartridge.ctg/validation/**. Historical reports that
refer to parent validation describe the old location; that directory moved into
source during cleanup.

- **Full build/check/test:** round-5-build.log, round-5-check.log and
  round-5-tests.log. The critic summed the Rust test-result records: **1,726 passed,
  zero failed, 17 ignored**. UI reports **34 passed, zero failed**. This broad run
  includes six tools tests; the final source-context fixture was subsequently
  covered by the separate seven-test tools run. Do not label 1,727 as an executed
  all-project aggregate. Runtime/ports strict Clippy, formatting, memory workspace
  checks and UI TypeScript checks pass. Earlier memory just check remains applicable
  to its unchanged Rust implementation.
- **Actual bundles:** round-5-bundle-source.log, round-5-bundle-wrapper.log and
  round-5-bundle-grouped.log record three successful Debug bundle contexts. The
  grouped invocation uses the genuine cartridge.ctg/workspace cwd. The source-direct
  log additionally records direct source invocation. Output is consistently
  cartridge.ctg/dist/cartridge; runtime, grouped ports and memory are built.
- **Relocation/completeness:** round-5-bundle-smoke.log records **15 manifests**,
  runtime plus **13 compiled cartridge executables**, all matching source target
  output hashes. Moved outside the checkout, the bundle's ledger succeeds and
  a real SDK **sessions.list returns []**. This tests executable relocation rather
  than merely finding cached files in the source tree.
- **Fresh local recursive clone:** round-5-recursive-clone.log records all 17
  submodules, clean clone status, the same metadata-only wrapper layout, 15 builtin
  links, and metadata counts of source **1**, grouped **13**, memory **28** packages.
  It uses explicit local URL overrides. Its runtime pointer 4acb234 differs from
  final 695adbb only by PORTING documentation, not executable/layout code.
- **New worktree:** round-5-lane-create.log, round-5-lane-links.log and
  round-5-lane-contract.log record a real disposable runtime worktree, **15 links /
  16 imported siblings** and **one tooling contract passed**. Parent confirms the
  temporary lane/branch was removed and all original 14 linked worktrees remain.
  Existing unrelated dirty runtime material was preserved.

Round-3 bundle omission and memory Docker sibling-mount findings stay resolved.
The memory recipes still mount /work/memory.ctg and read-only /work/cartridge.ctg,
with the former as cwd. Recipe expansion was tested; Docker daemon availability
prevented execution. No additional broad suite was rerun by this critic.

## Carried runtime measurements and profile changes

**No new full-port or runtime benchmark is claimed.** Runtime src and Cargo.lock
are unchanged since round-2 final b1a03ef. The only later Cargo change is:

```toml
[profile.dev]
debug = 0
incremental = false

[profile.test]
debug = 0
incremental = false
```

These settings reduce debug information/incremental-cache storage at the cost of
less debugger detail and potentially slower incremental rebuilds. They do not
change the release profile used by the benchmark. No measured build-time improvement
is claimed; build logs reuse cached dependencies and reflect different source paths.

[Round-2 raw evidence](round-2/results.json) remains the representative runtime
measurement: macOS 26.2 arm64, Rust/Cargo 1.98, Python 3.9.6, default release;
persistent Unix socket, 1,143-byte JSON payload, 320 warmups and 3,200 verified
responses per workload. Lua median/p95 is **0.024166/0.029458 ms** at one request
and **0.079917/0.139959 ms** at 16. SDK-to-host Lua plus two metadata callbacks:
**0.119333/0.147459 ms** at one and **1.151063/1.342791 ms** at 16. Startup uses
30 launches, zero excluded warmups: **8.712/12.257 ms** median/p95; first launch
257.677 ms, cache state uncontrolled. Daemon-only RSS is 11,568 KiB and excludes
child memory. Python contributes to timing/throughput. These figures do not measure
launcher overhead, full UI/agent/memory sessions, landscape Git collection or
sustained queue/history growth.

## Size evidence

Runtime release remains **5,782,800 bytes**, about 5.51 MiB, from preserved measured
release evidence. Runtime active source remains **7,683 production / 6,457 test
physical Rust lines**. Runtime dependency counts remain nine direct, 78 resolved
normal/build and 97 lock package records. Grouped lock has 243 records and memory
353; locks overlap and include target/dev records, so summing them would not be a
production dependency count. No new release image was built for this critique.

Final tools source totals **636 production / 292 test physical Rust lines**, up
13 / 65 from round 4. Thus the collection's active Rust estimate becomes **68,014
production / 61,736 test lines** under the same round-3 methodology: tracked active
Rust, excluding historical runtime copies, tests/fixtures/bench directories and
exact cfg(test) items separated; sessions test modules manually classified.
Comments and blank lines count. This is a syntactic estimate, not build reachability;
UI/Lua and other assets are outside Rust LOC. Moving orchestration files changes
ownership, not the amount of application functionality.

A fresh read-only count of the final Debug distribution at source/dist/cartridge
found **3,687 regular files / 270,518,666 bytes** (about 258 MiB), including installed
UI dependencies. Symlink entries are excluded; this is regular-file content size,
not disk allocation or archive size. Round 4 counted 270,495,930 bytes under the
same rule; this layout correction changes that observed debug total by 22,736 bytes.
That small comparison does not establish a full release footprint. No unnecessary
feature deletion or unsupported dependency removal is recommended merely to reduce
line/byte counts.

## Remaining limits and final judgment

No new blocking regression was found in the final layout. Nonblocking follow-ups
remain the same: measure real full-application sessions and full Release bundle
size, execute container/remote target checks when available, and preserve focused
path-selection tests when repository layout changes. Runtime steady-state frame
sizes, subscriber queues and replay history remain unbounded; future retention or
backpressure changes must preserve tested replay semantics. Landscape's per-entry
Git collection and broader imported code deserve workload-specific profiling before
making stronger performance claims.

Sandbox **macOS policy, Linux policy and launch-authority board work remain open**.
The command adapter does not make every launch confined. This critique opens no
new sandbox PRD and reproduces no new sandbox bypass. **Remote CI, publication,
actual Docker execution, full application Release bundle validation and full-port
performance measurement remain unrun.** Local recursive clone success with URL
overrides is not evidence that configured GitHub repositories were published.

The local final layout satisfies the requested wrapper/source ownership boundary,
and its focused and executed integration evidence supports **PASS: 82 / 90 / 86**.
This completes the fifth-round review; no sixth round is implied.
