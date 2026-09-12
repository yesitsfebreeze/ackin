# Whole-program critique — round 2 of 5

**PASS — Performance 84/100; cleanliness 90/100; size 88/100.**
All three dimensions meet the fixed 80 threshold. Scores are reviewer judgments,
not benchmark percentages. This review covers the standalone runtime measured at
`f22b0ea8af268c3e037389d36ea159f406d84230`, with final test-only corrections through
`b1a03ef0aa28c58cb9f2ed3209312f646c7f79f1`, including its current sandbox adapter,
SDK, Lua, loader/reload, resolver/ledger, streams, diagnostics, CLI and tests.
Parent repository integration and the later physical move are outside this round.
The critic wrote the measurement harness and evidence, but no runtime corrections.

## Fixed rubric, unchanged allocations

| Performance allocation | Maximum | Round 1 | Round 2 | Evidence and deductions |
|---|---:|---:|---:|---|
| Representative runtime latency/startup | 30 | 28 | 27 | SDK and concurrent Lua remain fast; single-request Lua median increased 5.42 microseconds. No supplied SLA; no causal attribution from one run. |
| Resource ownership, bounds and recovery | 30 | 19 | 25 | Discovery/readiness bounds, child ownership, interruptible retirement and deterministic socket cleanup now have regression coverage. Steady-state framing, queues and history remain unbounded. |
| Concurrency and algorithmic scaling | 25 | 13 | 20 | Completed-subtree memoization removes repeated DAG expansion while retaining active cycle checks; concurrent throughput remains useful. Ledger scans, shared execution and unbounded fan-out remain scaling costs. |
| Measurement and regression evidence | 15 | 12 | 12 | Same fixed release workload, raw samples, verified replies and complete declared source snapshot. No sustained-load or large-profile runtime measurements. |
| **Total** | **100** | **72** | **84** | |

| Cleanliness allocation | Maximum | Round 1 | Round 2 | Evidence and deductions |
|---|---:|---:|---:|---|
| Correctness and contract coverage | 35 | 26 | 32 | 148 passing tests cover old contracts plus F1–F4, reload registration and landscape. No newly demonstrated blocking regression; fault/platform coverage is not exhaustive. |
| Architecture and maintainability | 25 | 22 | 21 | Shared process ownership helps. Loader remains large; direct/nested startup still repeat protocol mechanics, and cancellation/connection/reload ownership now requires more careful reasoning. |
| Error handling and lifecycle clarity | 20 | 14 | 17 | Failed discovery is killed/reaped, diagnostic failed writes repair or disable, and socket work has explicit owners. The 100 ms cooperative inverse window and host-owned accepted reloads are deliberate lifecycle tradeoffs requiring preservation. |
| Formatting, lint and build hygiene | 20 | 8 | 20 | Default fmt and strict all-target clippy pass; locked tests/release build pass; final standalone workspace metadata verified. |
| **Total** | **100** | **70** | **90** | |

| Size allocation | Maximum | Round 1 | Round 2 | Evidence and deductions |
|---|---:|---:|---:|---|
| Production code proportional to requirements | 40 | 36 | 36 | Added bounds, lifecycle ownership, landscape and sandbox adapter explain growth; no required behavior was deleted to obtain a smaller count. |
| Avoidable duplication/unused structure | 25 | 20 | 21 | Shared process helper and corrected development watcher reduce waste; direct/nested protocol setup still repeats. |
| Dependencies and feature scope | 20 | 17 | 17 | Same nine direct, 78 resolved normal/build and 97 lockfile packages; tokio/full remains broader than demonstrated need. |
| Release artifact proportionality | 15 | 14 | 14 | 5.51 MiB, up 2.17%, remains proportionate to embedded Lua and the runtime/CLI; no supplied tighter artifact budget. |
| **Total** | **100** | **87** | **88** | |

## Reproducible measurements

Command, run from the main checkout:

```sh
CARGO_INCREMENTAL=0 python3 .pearde/quality-tools/measure.py /Users/feb/dev/cartridge /Users/feb/dev/cartridge/.pearde/quality-evidence/round-2 --target-dir /private/tmp/zirkle-quality-target-hgo93yb4
```

[Raw results](round-2/results.json) and adjacent logs preserve samples, environment,
build output, dependency metadata and source hashes. Completed 2026-09-12 21:22:15 UTC.
Same macOS 26.2 arm64 host, 10 logical CPUs, Rust/Cargo 1.98.0 and Python 3.9.6.
Default Cargo release, no extra RUSTFLAGS or stripping. Dependency artifacts were
reused; the 7.24-second release build is not a clean-build performance claim.

Protocol local-rpc-v1 is unchanged: persistent Unix socket, 1,143-byte JSON payload,
320 warmups and 3,200 verified replies per workload. SDK roundtrip includes host
Lua echo plus two metadata callbacks. Concurrency 16 uses batches on one connection.
Latency includes Python response decoding, excludes request encoding; throughput
includes encoding and validation. Package/binary/Lua identity changed from zirkle
to cartridge as requested; the payload and timing rules did not change.

| Path / concurrency | Baseline median / p95 ms | Round 2 median / p95 ms | Baseline → round 2 calls/s |
|---|---:|---:|---:|
| Lua / 1 | 0.018750 / 0.023208 | 0.024166 / 0.029458 | 38,525 → 30,993 |
| SDK → Lua / 1 | 0.118333 / 0.137375 | 0.119333 / 0.147459 | 7,866 → 7,701 |
| Lua / 16 | 0.079959 / 0.144750 | 0.079917 / 0.139959 | 88,414 → 85,903 |
| SDK → Lua / 16 | 1.167855 / 1.365625 | 1.151063 / 1.342791 | 11,535 → 11,726 |

Startup, 30 launches with zero excluded warmups: median/p95 **8.712/12.257 ms**,
versus **8.817/15.167 ms**. First launch remains the maximum, **257.677 ms** versus
270.835 ms. Readiness polling is 1 ms, caches uncontrolled, predominantly warm
repeat launches. This does not establish a cold-start improvement.

The Lua/1 increase is real in the recorded samples (median +28.9%, absolute
+5.42 microseconds), but a single short run cannot isolate code overhead from
scheduler/host/client variation. Concurrent Lua is essentially unchanged in median;
SDK results do not show a comparable shift. Do not call this a universal speedup
or a statistically established regression. Child/task cleanup and DAG bounds earn
performance points independently of these latency comparisons.

Daemon-only RSS after workload: **11,568 KiB**, baseline 11,536 KiB. Release binary:
**5,782,800 bytes**, baseline 5,660,224, a **122,576-byte / 2.17%** increase.
Nine direct dependencies, 78 resolved normal/build packages, 97 total lockfile
packages including root/dev/other targets, all unchanged.

Test-aware Rust physical LOC: **7,683 production / 6,450 tests and fixtures**,
versus corrected baseline **6,786 / 5,409** (+897 / +1,041). Nonblank counts:
**7,286 / 6,199**, baseline **6,430 / 5,185**. Counts include comments. Entire
src/tests files and complete exact #[cfg(test)] items elsewhere count as tests;
string/comment-aware item boundaries and per-file ranges are recorded. No compound
test cfg needed manual classification. Historical baseline JSON is unchanged;
its original directory-only 6,830/5,365 is not the comparison denominator.

## Source and validation correspondence

All **47** entries in [validation source hashes](round-2-validation/source-hashes.json)
matched the measured checkout; final test-only differences are recorded below. The harness enumerated **48** files before and after
measurement, including the additional untracked src/tests/.DS_Store; no source
files were added, removed or modified during measurement. The runtime diff was empty at measurement time.
Hash scope includes all src files/fixtures, .cargo and declared root Cargo/build/fmt
inputs. It excludes external path dependencies, toolchain/user configuration,
generated OUT_DIR and assets outside that scope; it is not a hermetic build hash.

Preserved [gate results](round-2-validation/gates.json) and tests.log show **148
passed, zero failed/ignored**, fmt pass and strict all-target clippy pass. These
checks ran on fd8e132; f22b0ea adds only the empty standalone [workspace] and comment.
Final metadata and this release build validate that boundary. No broad check rerun
was needed. Reported MSRV 1.89 and non-native targets were not separately tested.

## Disposition of required round 1 findings

- **F1 fixed:** src/turn.rs:109 recurses into arrays; :175 substitutes valid bounded
  omission records before rotation; :215 repairs partial failed writes or disables
  the sink. Tests at :334 and :343 cover nested synthetic credentials and multibyte
  initial/rotated oversized records. The unrecoverable sink test also passes.
- **F2 fixed within the requested startup scope:** src/process.rs:7 sets five-second
  startup and 64 KiB discovery budgets; :36 owns child/task teardown; :65 captures
  discovery concurrently and kills/reaps errors; :103 bounds even unterminated
  startup frames. Async SDK discovery awaits directly (src/sdk.rs:222), loader
  composition uses the blocking pool (src/loader.rs:1155). src/runtime.rs:395 can
  stop never-yielding work while briefly accepting its pending inverse. Regression
  assertions cover hung/excessive hello, unready direct child timeout/disposal,
  nested cancellation, and prior-inverse execution (src/tests/process.rs:600,
  :612; src/tests/lifecycle.rs:387, :410). Ordinary RPCs retain duration semantics.
- **F3 fixed on the tested native platform:** src/socket.rs:90 owns subscription
  identifiers and pumps, :113 owns connection tasks, :190 separates accepted host
  reload from its reply waiter. Tests at :319, :428, :458 and :499 cover idle/full
  disconnect, pending-call cancellation, half-close reply delivery and cleanup
  during an accepted blocked reload; explicit unsubscribe is also tested.
- **F4 fixed:** src/resolver.rs:98 skips completed provider paths only after active
  cycle detection; :106 records completion after dependencies. Per-call scope
  avoids stale persistent caching. src/tests/resolver.rs:631 covers 24 shared
  layers / 47 unique providers and dependency order; existing scope/cycle tests pass.
- **F5 fixed:** rustfmt.toml states intended tabs; preserved fmt/clippy gates pass.
  The stale core watcher now names src. No global lint suppression was used.

Requested rename, Value-based SDK Host::on_reload (src/sdk.rs:72) and landscape
(src/landscape.rs:36; src/sdk.rs:67) are present with dispatch/composition tests.
No compatibility aliases are required by the explicit rename instruction.

## Prioritized nonblocking follow-ups and remaining limits

1. **Performance/resources:** src/socket.rs:115, :163; src/stream.rs:49, :92;
   src/cartridge.rs:349, :423; src/sdk.rs:240, :309. Steady-state lines, queues and
   replay retention can still grow with input or a slow consumer. Add sustained
   slow-reader and large-frame measurements before choosing admission/backpressure
   and retention policies. Preserve the tested replay contract; silent truncation
   would be a behavioral regression. Startup bounds are not a whole-session bound.
2. **Performance/portability:** src/socket.rs:126 probes each idle connection every
   100 ms with zero-byte writes. Native tests substantiate the intended full-close
   versus half-close behavior on macOS; they do not prove it on every Unix target
   or quantify many-idle-connection overhead. Validate those cases before claiming
   portable cleanup or high connection-count efficiency.
3. **Cleanliness/size:** src/loader.rs:720, :1155, :1259; src/cartridge.rs:319;
   src/sdk.rs:207. Keep shared child ownership, but consider extracting common
   protocol startup mechanics when another feature touches these sites. Preserve
   accepted reload transaction completion and the documented cooperative inverse
   window. No unrelated architectural rewrite is necessary for this pass.
4. **Size:** Cargo.toml:39 still enables tokio/full. Explicit used features could
   clarify dependency intent; no measured release saving supports a removal claim.

The sandbox command adapter is now present, with native denial tests and explicit
Linux refusal. It is not yet wired through all discovery/direct/nested/chain
launches; trusted spawning/broker and Linux confinement remain unfinished PRD
work, as in round 1. This pass does not certify OS confinement or upgrade a grant
into enforcement. No new sandbox bypass was dynamically reproduced by this critic.

No sustained stress, leak trend, full agent session, large-profile runtime timing,
external-network workload or cross-platform benchmark was run. The Python driver
contributes to measured latency/throughput, daemon RSS excludes child memory, and
no profile isolates SDK scaling costs. These limits remain deductions rather than
claims of proved absence of bottlenecks.

## Late validation evidence — resolved

Parent reports a later staging runtime run with 147 passed and one failure in
`src/sdk.rs:668`: the new cancellation fixture did not observe its PID file within
a one-second startup wait under contention. The proposed correction increases
fixture startup/reap waits to three seconds and polls with a 10 ms sleep, retaining
assertions that the child actually dies and leaving production deadlines unchanged.
The patch, focused rerun and final actual-main all-target rerun have been reviewed.
The existing measured candidate and its preserved 148-pass gate remain accurately
reported above; they do not negate the later flaky-test observation.

The final correction at b1a03ef waits up to three seconds for a parsed positive
u32 PID, retains it before abort, polls every 10 ms, and still checks cancellation
and subsequent OS process absence within three seconds. This closes both the
contention sensitivity and a potential false pass from reading an empty PID file.
Only src/sdk.rs differs from the measurement snapshot, entirely inside cfg(test);
its final SHA256 is
`b01b89c4363fe07dd13f3410f16d848763c969d45ce3b757cfcace48cff710a9`.
Final production LOC remains 7,683; test LOC becomes 6,457 physical / 6,206 nonblank
(+7 from the measured candidate). Runtime build inputs outside test code are
unchanged, so the fixed release measurements remain applicable. The focused
staging cancellation-final.log passes.

Final actual-main gates at b1a03ef, recorded in
[final gate results](round-2-validation/final-main-gates.json),
[tests](round-2-validation/final-main-tests.log),
[fmt](round-2-validation/final-main-fmt.log) and
[clippy](round-2-validation/final-main-clippy.log): **148 passed, zero failed/ignored**
(7.89 seconds library execution); fmt exit 0; strict all-target clippy exit 0.
The corrected assertion still tests actual child cleanup. The test-only amendment
does not change the scores or require repeating the production benchmark.
Final verdict: **PASS, 84 / 90 / 88**.
