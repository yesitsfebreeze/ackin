# Whole-program critique — round 1 of 5

**FAILURE — Performance 72/100; cleanliness 70/100; size 87/100.**
Every dimension must reach 80. These are reviewer judgments, not benchmark
percentages. Main checkout commit: `92284ca5623bd95be073575ade146f6ea3e8706a`.
Runtime/test/manifest hashes still match baseline. The in-flight sandbox lane
is excluded. No completed baseline checks were rerun and no runtime was edited.

The measurement worker performed this subsequent critique independently of
implementation. Review covered runtime module boundaries, loader/reload,
fiber lifecycle, resolver/ledger, socket transport, SDK, Lua bridge, stream,
diagnostics, existing tests, manifest and sandbox PRD/report. New defects below
are source-derived findings, not newly executed regression tests.

## Fixed rubric for all subsequent rounds

Retain these allocations and the baseline benchmark workload. Related findings
are assessed together within a category, not charged repeatedly per symptom.

| Performance allocation | Maximum | Round 1 | Evidence |
|---|---:|---:|---|
| Representative runtime latency/startup | 30 | 28 | Fast verified local RPC; no supplied SLA establishes a stricter target. |
| Resource ownership, bounds and recovery | 30 | 19 | Cancellation-aware Link and bounded child disposal are strengths; discovery, pre-ready waits, oversized records and socket pumps have gaps. F1–F3. |
| Concurrency and algorithmic scaling | 25 | 13 | Concurrent local throughput improves; SDK rises about 1.47× at 16 in flight. Resolver re-expands shared subtrees; queues/history remain unbounded. F4 and limitations. |
| Measurement and regression evidence | 15 | 12 | Release, fixed payload, raw samples, warmups, verified replies and hashes; short client-limited workloads, no sustained/large-profile measurements. |
| **Total** | **100** | **72** | |

| Cleanliness allocation | Maximum | Round 1 | Evidence |
|---|---:|---:|---|
| Correctness and contract coverage | 35 | 26 | 119 tests pass, including nested RPC/reload/JSON/scopes; diagnostic counterexamples escape existing tests. F1. |
| Architecture and maintainability | 25 | 22 | Useful Link/Reload/Service/ledger boundaries; loader combines format, composition, contracts and transactions; startup mechanics repeat. |
| Error handling and lifecycle clarity | 20 | 14 | Typed errors and EOF rejection are sound; discovery/readiness and detached socket lifetimes remain incomplete. F2–F3. |
| Formatting, lint and build hygiene | 20 | 8 | Release and all-target tests pass; default fmt and strict clippy fail. F5. |
| **Total** | **100** | **70** | |

| Size allocation | Maximum | Round 1 | Evidence |
|---|---:|---:|---|
| Production code proportional to requirements | 40 | 36 | About 6.8k production Rust lines implement substantial required behavior; tests are separate. |
| Avoidable duplication/unused structure | 25 | 20 | Shared RPC bookkeeping is good; direct/nested startup repeat mechanics; stale core watcher convention. |
| Dependencies and feature scope | 20 | 17 | Nine direct dependencies have identifiable uses; tokio/full is broader than demonstrated use; no removal saving measured. |
| Release artifact proportionality | 15 | 14 | 5.40 MiB includes embedded Lua and runtime/CLI facilities; no contrary size budget. |
| **Total** | **100** | **87** | |

## Measured evidence

Command: `python3 .pearde/quality-tools/measure.py /Users/feb/dev/cartridge .pearde/quality-evidence/baseline --checks`.
Full evidence: [baseline report](baseline/README.md),
[raw results](baseline/results.json), and adjacent stdout/stderr logs.
macOS 26.2 arm64, 10 logical CPUs, Rust/Cargo 1.98.0, Python 3.9.6;
default Cargo release, no extra RUSTFLAGS or stripping; private target
`/private/tmp/zirkle-quality-target-hgo93yb4`.

Persistent Unix socket, 1,143-byte JSON payload, 320 warmups and 3,200 verified
replies per workload. SDK calls include Lua echo and two metadata callbacks.
Concurrency 16 uses batches on one connection. Response decoding is inside
latency; request encoding is inside throughput but outside individual latency.

| Path | Concurrency | Median ms | p95 ms | Completed calls/s |
|---|---:|---:|---:|---:|
| Lua | 1 | 0.01875 | 0.02321 | 38,525 |
| SDK → Lua | 1 | 0.11833 | 0.13738 | 7,866 |
| Lua | 16 | 0.07996 | 0.14475 | 88,414 |
| SDK → Lua | 16 | 1.16785 | 1.36563 | 11,535 |

Startup: 30 samples, zero excluded startup warmups, median 8.817 ms,
p95 15.167 ms, first-launch maximum 270.835 ms. Caches uncontrolled;
subsequent launches predominantly warm. Daemon RSS after work: 11,536 KiB,
excluding SDK child. Binary: **5,660,224 bytes**, approximately **5.40 MiB**.
Nine direct dependencies; 78 resolved normal/build packages on this host;
97 lockfile packages including root/dev/other targets.

Baseline directory-based production/test Rust physical LOC: 6,830/5,365.
There are 44 test-only lines outside src/tests: src/turn.rs:257 (38),
src/cartridge.rs:177 (4), src/lib.rs:16 (2). Corrected production/test
physical LOC: **6,786/5,409**; corrected nonblank LOC: **6,430/5,185**.
Counts include comments. Carry this test-aware classification to later rounds.

Release builds. `cargo test --locked --all-targets`: **119 passed**, zero
failed/ignored. `cargo fmt --all -- --check` fails with widespread differences:
tab-based source has no matching configuration. Strict all-target clippy fails
at src/tests/wire.rs:163 on `unnecessary_to_owned`.

## Prioritized fixes

### F1 — High: structured diagnostics bypass redaction through arrays and exceed the cap

Anchors: **src/turn.rs:109**, **src/turn.rs:168**; tests at src/turn.rs:261
and src/turn.rs:277. Affects cleanliness and performance/resource bounds.
Evidence is deterministic source inspection, not a separately executed probe.

`redact` returns immediately unless its input is an object. For
`{"events":[{"api_key":"synthetic-token"}]}`, recursion reaches the array
and leaves the nested credential intact. Recurse into arrays and test nested
arrays with synthetic data. This is a structured diagnostic redaction hole,
not an OS sandbox escape.

`Sink::write` rotates only when a previous record exists, then writes the
entire next record. A first record larger than cap exceeds it immediately;
after rotation an oversized record still exceeds it. Define an oversized-record
policy that preserves valid JSON and identifies omitted/truncated content.
Test empty and rotated files, including multibyte text. Preserve useful logs.

### F2 — High: discovery and pre-ready startup can wait without a bound

Anchors: **src/cartridge.rs:280**, **src/cartridge.rs:365**,
**src/sdk.rs:212**, **src/sdk.rs:251**, **src/loader.rs:1086**,
**src/runtime.rs:395**. Affects performance and cleanliness. Source inspection.

Discovery uses synchronous `Command::output()` without timeout or output cap.
A never-exiting hello prevents composition returning; the SDK also blocks an
async executor thread. Foreground readiness timeout begins after reconcile,
so it does not cover discovery. Infinite captured output can exhaust memory.
Direct/nested loops waiting for ready have no local timeout. The runtime's
execute guard is checked before awaiting the next stream item, so retirement
cannot interrupt an apply that never yields. The later foreground timeout
therefore does not ensure disposal finishes.

Bound discovery duration and captured output, kill/reap failed discovery
children, keep blocking discovery off the executor, and make pre-ready work
terminate on timeout/owner disposal. Preserve legitimate long-running RPCs;
a blanket service-call deadline is not the fix. Verify hung hello, excessive
hello output and a child remaining alive without ready; assert cleanup and
bounded disposal. Reuse a shared lifecycle primitive if broker work replaces
these spawn sites.

### F3 — Medium: socket subscriptions lack deterministic disconnect cleanup

Anchors: **src/socket.rs:147**, **src/socket.rs:160**,
**src/socket.rs:190**, **src/socket.rs:205**. Affects performance and
cleanliness. Source inspection; existing tests do not cover idle disconnect.

A subscription pump owns the Stream while awaiting its receiver. After socket
closure it notices a missing weak sender only when another channel event
arrives. An idle channel can retain the pump/stream indefinitely; the connection
never drains its subscription IDs. The send check tests only Option::is_none,
so Some(Err(SendError)) is treated as success while another in-flight call still
owns a strong sender. RPC tasks are detached from connection ownership.

Explicitly unsubscribe connection IDs and terminate pumps on teardown. Treat
failed sends as failures and give connection tasks a defined lifetime, preserving
half-close behavior where supported. Verify idle disconnect, writer failure
while another RPC is pending, and explicit unsubscribe, without relying on a
future publish to clean up.

### F4 — Medium: resolver repeats completed subtrees before deduplicating output

Anchors: **src/resolver.rs:86**, **src/resolver.rs:93**,
**src/resolver.rs:98**; scope scans at src/ledger.rs:143 and :184.
Affects performance. Algorithmic source analysis, not measured by RPC benchmark.

Every visit descends through a provider's needs before checking whether the
provider is already in the result. Layered diamonds can therefore expand the
same small DAG once per incoming path, giving exponentially many visits in the
number of layers; each visit also performs ledger scope scans.

Track completed provider paths separately from the active cycle stack. Skip
re-expansion after successful completion while preserving bottom-up order and
cycle/ambiguity/unbound diagnostics. Verify shared diamonds and layered sharing.
Avoid a speculative persistent cache that compromises fresh ledger resolution.

### F5 — Medium: standard fmt and clippy gates fail

Anchors: **src/tests/wire.rs:163**, formatting beginning at
**src/cartridge.rs:35**, lint settings **Cargo.toml:51**.
Affects cleanliness. Executed baseline evidence.

Pass key directly rather than &key.to_owned(). Configure rustfmt for intended
project style, format, then pass the same default fmt and strict all-target
clippy commands. Do not suppress the lint globally. Existing gate reruns are
sufficient; formatting does not need new behavioral tests.

## Secondary size/architecture observations

No broad deletion is justified: Lua, async RPC, nested SDK composition,
scopes, hot replacement, contract verification and diagnostics have demonstrated
requirements. The release image is proportionate and every direct dependency
has a source use. Direct/nested launch, stderr, writer, readiness and disposal
repeat at src/cartridge.rs:319 and src/sdk.rs:207. Consolidate when F2/broker
work already touches ownership, rather than creating a generic framework to
save lines. Tokio full at Cargo.toml:39 includes fs with no current tokio::fs
source use; explicit features could clarify intent, but no binary saving is
claimed. The watcher at src/loader.rs:1678 names core, absent from this checkout;
remove or explain that obsolete development convention when updating watcher
configuration. These secondary refinements do not justify losing features.

## Sandbox scope and limits

The sandbox PRD calls OS confinement new, unfinished work. Main launches hello
(src/cartridge.rs:284), direct processes (:327), SDK children (src/sdk.rs:214),
and chain nodes (src/main.rs:419 and :850) without a policy wrapper. Lua is
evaluated in host at src/lua.rs:271. Grant data is not confinement. This is a
pending delivery gap, not a regression against an already shipped wall.

The PRD's recorded macOS experiments show different policies cannot be applied
inside a confined parent; the delegated choice requires trusted host spawning
from host-resolved installed identities. The recorded network choice explicitly
opens networking when net is nonempty. That accepted platform limit must not
be hidden or called a new hostname-policy bypass. Later critique must inspect
discovery and nested/chain entry paths as well as direct spawning and reject
caller-supplied replacement policy/arbitrary broker commands. **No new OS sandbox
bypass was dynamically reproduced in this critique.** Lane evidence was not
claimed as main evidence. Linux confinement/cross-target builds are unvalidated.

Stream history is deliberately retained (src/stream.rs:49), with unbounded
subscriber queues. This carries scaling costs; the short benchmark establishes
no long-run memory bound. Silently discarding replay history would change a
tested contract. A later retention/backpressure design must preserve replay or
explicitly account for gaps; that redesign is not required to fix F1–F5.

No profile attributed SDK concurrency behavior to a specific hotspot: callback
count, client work, scheduling and shared Lua can contribute. No sustained load,
large-profile, external-network tool session or cross-platform benchmark ran.
The fixed workload must be reused after changes, with focused checks for the
findings. Fixing this list does not automatically guarantee a later score of 80.
