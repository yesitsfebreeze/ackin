# Whole-program critique — round 4 of 5: corrected integration

**PASS — Performance 82/100; cleanliness 89/100; size 86/100.**
All dimensions meet the fixed 80 threshold. Scores are reviewer judgments, not
benchmark percentages. Reviewed parent **0f867534421e3335788c4e1ffb589d22a4a0ca71**,
tools **56165b2614df5d00d0878613e53b78f95d59089e**, memory
**ddd479766c8a8a365d8df7abcc35ec8f4c212317**. Runtime production remains identical
to round-2 final b1a03ef; this correction changes tools build/package code and
memory container recipes. The critic edited no implementation files.

This is the fourth actual critique round. It evaluates the corrected integration,
including real debug bundles from both working-directory contexts, following the
round-3 failure. It does not reopen unrelated sandbox work or imply that the full
application received a new performance benchmark.

## Fixed rubric — allocations unchanged

| Performance allocation | Maximum | Round 3 | Round 4 | Evidence and deductions |
|---|---:|---:|---:|---|
| Representative runtime latency/startup | 30 | 27 | 27 | Same production source supports the scoped round-2 benchmark. Whole-port startup, agent/UI/memory sessions and launcher overhead remain unmeasured. |
| Resource ownership, bounds and recovery | 30 | 25 | 25 | Runtime ownership fixes retained; build plan uses one explicit target/profile across separate workspaces. Runtime queues/history and steady-state framing remain unbounded. |
| Concurrency and algorithmic scaling | 25 | 19 | 19 | Completed resolver-subtree handling retained. Per-directory Git landscape collection and full application concurrency remain unprofiled. |
| Measurement and regression evidence | 15 | 10 | 11 | Actual bundles from both contexts and an isolated SDK smoke strengthen artifact-level evidence. No sustained/full-port benchmark or release-build timing is substituted. |
| **Total** | **100** | **81** | **82** | |

| Cleanliness allocation | Maximum | Round 3 | Round 4 | Evidence and deductions |
|---|---:|---:|---:|---|
| Correctness and contract coverage | 35 | 23 | 32 | Both required bundle corrections are exercised by focused tests and actual builds; all expected executable hashes match and relocated SDK/ledger smoke succeeds. Not every cartridge operation was executed. |
| Architecture and maintainability | 25 | 21 | 21 | Metadata-based BuildPlan centralizes required workspaces and command arguments. Parent helper and tooling still encode related layout knowledge; extraction is not required for this fix. |
| Error handling and lifecycle clarity | 20 | 17 | 17 | Missing required binaries now fail with their path before publication; existing runtime lifecycle evidence carries. No new lifecycle regression found. |
| Formatting, lint and build hygiene | 20 | 15 | 19 | Focused tools tests, fmt and strict Clippy pass; both actual build contexts work. Docker recipes now express the right paths, but daemon execution remains unavailable. Remote CI is unrun. |
| **Total** | **100** | **76** | **89** | |

| Size allocation | Maximum | Round 3 | Round 4 | Evidence and deductions |
|---|---:|---:|---:|---|
| Production code proportional to requirements | 40 | 35 | 35 | A bounded build-plan correction preserves the imported functionality; no arbitrary deletion. The large inherited memory implementation remains integration-reviewed rather than exhaustively audited for dead code. |
| Avoidable duplication/unused structure | 25 | 21 | 21 | Workspace discovery is shared between contexts and uses metadata instead of textual [workspace] detection. Existing parent/helper duplication and preserved historical evidence remain. |
| Dependencies and feature scope | 20 | 17 | 17 | No dependency/lock changes in this correction; independent memory SDK feature boundary retained. Earlier tokio/full observation remains. |
| Release artifact proportionality | 15 | 12 | 13 | Complete debug artifact is now available and counted, and runtime release remains 5.51 MiB. Complete application release footprint is still unmeasured; debug bytes are not release bytes. |
| **Total** | **100** | **85** | **86** | |

## Required findings resolved

**R3-F1 — resolved.** At tools.ctg/service.rs:128, BuildPlan starts with Cargo
metadata, finds the runtime composition and, in the composed layout, consistently
uses parent target/dist configuration. It includes parent, canonical runtime and
independent memory workspace manifests. Known member manifests and workspace roots
are deduplicated. At :182, each build receives its explicit manifest, the same
target directory and selected profile; :318 packages from that completed plan.
Source inspection found no path that recreates the earlier parent/runtime omission
for the shipped layout.

At :238, package acceptance requires an existing binary when a manifest declares
one or the cartridge has a Cargo manifest. The error names the missing binary and
cartridge directory. Lua-only cartridges remain accepted. The runtime executable
copy is still mandatory. This closes the previous silent incomplete-bundle path.

Focused tests at :622 exercise parent/runtime contexts for both Debug and Release
command plans, assert all three workspace roots and explicit target/profile flags.
The :672 fixture preserves flat-workspace compatibility. The :687 test refuses
missing implicit Rust and explicit custom binaries, permits Lua-only content, and
then checks a successful explicit-binary copy. Existing composition/lane/package
checks also pass: **six tools tests**, fmt and strict Clippy green, as reported
by the implementation worker. These command-plan tests supplement rather than
replace the actual debug builds below.

**R3-F2 — resolved at source/layout level, execution limitation retained.**
Memory commit ddd4797 updates both `docker` and `docker-test` in memory.ctg/justfile:
memory is mounted at **/work/memory.ctg**, its SDK at **/work/cartridge.ctg:ro**, and
the working directory is **/work/memory.ctg**. This supplies the exact sibling
manifest required by Cargo.toml's ../cartridge.ctg dependency, including optional
feature resolution. It preserves read-only SDK source and memory's own working
scope. The generated commands in validation/memory-docker-recipes.log match those
paths. validation/docker-availability.log records that the Docker daemon socket
was unavailable; no container execution is claimed. Remote workflow execution and
Windows portability remain unvalidated rather than assumed.

## Actual bundle evidence

Parent validation logs, preserved locally:

- `validation/round-4-tools-build.log`: rebuild of the corrected tools executable.
- `validation/round-4-bundle-runtime.log`: actual Debug bundle invoked from runtime
  context; parent cartridges, runtime and memory all build; UI production
  dependencies install; output is the parent dist/cartridge directory.
- `validation/round-4-bundle-parent.log`: actual Debug bundle invoked from parent
  context, reaching the same distribution directory and all three build roots.
- `validation/round-4-bundle-smoke.log`: **15 cartridge manifests**, runtime plus
  **13 compiled cartridge executables**, all present with hashes matching built
  outputs. Bundle relocated outside the workspace to a temporary directory;
  its ledger succeeds and a real SDK **sessions.list returns []**. Bundle restored
  to parent dist/cartridge afterward.

Both invocations completed successfully. Logs include cached development builds;
they are not controlled cold-build timings. Relocation and a real process RPC
provide stronger evidence than merely listing build outputs. They do not establish
all model-provider operations, UI rendering, memory retrieval, remote access or
platform support. Release command generation is tested, but **an actual full
Release bundle was not run**.

Round-3 broader evidence still applies to unchanged code: 1,723 Rust tests passed,
zero failed, 17 existing memory tests ignored; UI 34 passed. Strict runtime/ported
cartridge checks, memory check and separate just check passed. Local recursive
clone via explicit URL overrides verified 17 submodules, 15 links and all three
workspace roots; runtime worktrees were repaired and inherited dirty material
preserved. This round did not repeat those broad checks without a new failure.
The corrected tools tests add three cases; no new all-project aggregate run is
claimed by simply adding three to the earlier test count.

## Scoped performance and size evidence

Runtime `git diff b1a03ef HEAD -- src Cargo.toml Cargo.lock` is empty. Carried
[round-2 raw measurements](round-2/results.json): macOS 26.2 arm64, default release,
Rust/Cargo 1.98, Python 3.9.6; persistent Unix socket with 1,143-byte JSON,
320 warmups and 3,200 verified replies per workload. SDK-to-host Lua plus two
metadata callbacks: median/p95 **0.119333/0.147459 ms** at one request and
**1.151063/1.342791 ms** at 16. Lua-only: **0.024166/0.029458 ms** at one and
**0.079917/0.139959 ms** at 16. Startup 30 samples, zero excluded warmups:
**8.712/12.257 ms** median/p95; uncontrolled cache state and first launch included.
Daemon-only RSS 11,568 KiB. No whole-port latency or new benchmark is claimed.

Runtime release remains **5,782,800 bytes**, with **7,683 production / 6,457 test
physical Rust lines**. Nine runtime direct dependencies, 78 resolved normal/build
packages, 97 runtime lock records; parent 243 and memory 353 lock records remain
unchanged and overlapping. Their sum is not a production dependency count.

The corrected tools source contains **623 production / 227 test physical lines**
(main.rs plus service.rs), versus **542 / 112** before correction: **+81 production,
+115 test**. Counts include comments and blank lines, classifying exact cfg(test)
modules separately. The active collection estimate therefore becomes **68,001
production / 61,671 test Rust lines**, using the round-3 scope/methodology and
manual sessions classification. This is not a reachability analysis; historical
runtime Rust copies and non-Rust UI/Lua are excluded from Rust LOC.

A read-only count of the actual restored **Debug** bundle found **3,687 regular
files / 270,495,930 bytes** (about 258 MiB), including its installed UI dependencies.
It excludes symlink entries, so is regular-file content size rather than filesystem
allocation or archive size. Its debug runtime alone is 21,598,896 bytes; that is
not the 5,782,800-byte standalone release image. The bundle's 14 executable files
include the memory adapter and all other expected compiled cartridges. No claim
is made that every dependency byte is necessary or that release has this footprint.
The debug artifact is useful evidence of completeness; a release size budget would
require a separate actual release build and measurement.

## Remaining follow-ups and limits

No additional blocking regression was found in these corrections. The following
remain nonblocking limitations, rather than implied completed work:

1. Execute the Docker recipes when a daemon is available; keep the current explicit
   sibling mounts. Test remote CI and cross-target builds after publication.
2. Measure the full application release bundle and representative sustained agent,
   memory and UI sessions before making whole-program performance/size promises.
   Preserve replay semantics when addressing runtime backpressure/retention.
3. Keep focused tests around shared target/profile and workspace discovery when
   parent layout or build helpers change. Do not remove necessary modules merely
   to reduce the expanded application's line count.

Sandbox **macOS policy, Linux policy and launch-authority board work remain open**.
The command adapter is not comprehensive launch confinement, and no new sandbox
bypass was reproduced here. No new sandbox PRD was opened for this review.
**Remote CI/publication, actual Docker execution and full-port Release/performance
validation remain unrun.** The successful recursive clone used local URL overrides;
it did not prove GitHub publication. This pass evaluates the local corrected
integration under those stated limits.

Final verdict: **PASS — 82 / 89 / 86**.
