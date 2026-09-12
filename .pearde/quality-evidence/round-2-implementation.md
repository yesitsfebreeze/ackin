# Runtime correction iteration 2

Implemented F1–F5, the cartridge rename, Value-based SDK reload registration,
and the supplied landscape restoration. Committed on `correction/quality-runtime`:

- `fd8e132` — Rename runtime to cartridge and bound startup and connection lifetimes.
- `f22b0ea` — Keep cartridge runtime in its own Cargo workspace.

The lane is clean. No main runtime files, sibling lanes, historical Pearde
records, or benchmark harness were edited by this worker.

## Changes and regression evidence

**F1 — diagnostics.** Redaction descends through arbitrarily nested arrays.
Oversized records become a valid JSON omission record with original byte count;
smaller budgets use a minimal omission marker, and a budget too small to encode
that marker admits no record. Rotation operates on the replacement size.
Failed writes truncate back to the last complete record and reset the cursor;
a sink that cannot repair itself stops accepting records. Tests cover nested
synthetic credentials, multibyte oversized records in initial/rotated files,
and disabling an unrecoverable sink.

**F2 — discovery, readiness and retirement.** Shared process ownership aborts
pipe tasks and kills/reaps children when startup or the owner is dropped.
Discovery allows five seconds and at most 64 KiB on each captured stream;
it reads stdout/stderr concurrently. Synchronous composition runs that async
discovery in an isolated runtime, async SDK discovery awaits it directly,
and async loader paths move synchronous composition onto the blocking pool.
Pre-ready startup has a five-second deadline and a 64 KiB total stdout frame
budget, including unterminated frames. Retirement interrupts an apply/effect
that never yields after a 100ms cooperative window for its pending inverse.
Ordinary RPC calls retain their existing duration semantics.

Regression tests prove hung/excessive stdout/excessive stderr discovery is
refused and reaped, oversized unterminated startup output is rejected,
pre-ready direct disposal finishes promptly, an alive child without ready
is timed out and reaped, nested startup cancellation reaps its child, and
never-yielding apply/effect disposal finishes while prior inverses run.
The discovery test deadline is three seconds to include cold native shell
startup; its earlier 150ms fixture deadline proved too short under concurrent
compilation. Production discovery remains five seconds.

**F3 — socket ownership.** The server owns client tasks; a client owns RPC and
subscription tasks. Explicit unsubscribe aborts its pump and announces removal;
disconnection drains subscription IDs without waiting for another publication.
Failed channel sends are failures. Native zero-byte writes detect full peer
closure while preserving write-half-close replies. Reload transactions become
host-owned when accepted: disconnect cancels the reply waiter while the
transaction completes under the host reload lock, rather than aborting between
reload begin and finish.

Five native socket tests cover idle disconnect, explicit unsubscribe,
full disconnect with a forever-pending RPC, write-half-close with its delayed
reply, and subscription cleanup while reload is blocked followed by successful
transaction completion after disconnection. Their assertions also verify
pending Link waiter removal and single unsubscribe announcements.

**F4 — resolver.** A per-resolution completed set skips already-expanded
providers after the active-stack cycle check. Bottom-up order and existing
unbound/ambiguity/cycle diagnostics remain. A 24-layer shared DAG regression
checks 47 unique providers in dependency order and bounded completion; existing
resolver tests also pass. No persistent cache was introduced.

**F5 — formatting and lint.** Added rustfmt's explicit tab configuration,
formatted all runtime/tests, removed the owned-string comparison lint and
sandbox PathBuf comparison allocation, and changed the stale development
watcher path from core to src. Strict all-target clippy has no warnings.

**Rename and restored APIs.** Package, binary/default-run, lockfile, Rust imports,
Lua API, .cartridge paths, environment names, wire labels, source documentation,
and fixtures now use cartridge. No legacy aliases were added. SDK
`Host::on_reload` has the original `Fn(Value) -> Future<Result<Value>>` shape
and inserts its boxed handler under reload. Its dispatch test checks both
boolean Values used for prepare/cancel. The supplied landscape module, SDK
request, nested relay, host response, and two committed-composition tests are
integrated; provider settling waits from the parent were included.

## Final validation

All builds used `CARGO_INCREMENTAL=0` and this lane's target directory.
One build ran at a time. No generated artifacts were broadly deleted.

| Command | Exit | Result |
|---|---:|---|
| cargo fmt --all -- --check | 0 | Clean |
| cargo clippy --locked --all-targets -- -D warnings | 0 | No warnings |
| cargo test --locked --all-targets | 0 | 148 passed, 0 failed, 0 ignored; 7.71s library execution |
| cargo metadata --locked --no-deps --format-version=1 | 0 | This runtime directory is the sole standalone workspace member |

The all-target gate also ran the binary and four fixture targets (zero unit
tests each). Final test compilation took 2.72s. `git diff --check` passed.
An old-name search over src and Cargo inputs found no zirkle/ZIRKLE/Zirkle.

The first three gates ran against `fd8e132`. The parent requested the empty
`[workspace]` immediately after that commit; `f22b0ea` adds only that standalone
workspace boundary and its comment. Locked metadata proves the intended root
and sole member. Unchanged behavioral tests were not repeated for this
manifest-only isolation change.

Durable evidence is in `round-2-validation/`: `gates.json`, `fmt.log`,
`clippy.log`, `tests.log`, `metadata.json`, `source-hashes.json`, and focused
regression logs. Source hashes describe the final two-commit candidate.
Earlier failing exploratory logs are clearly separate from the final gate.

This report supplies implementation and test evidence, not critique scores.
The fixed benchmark and independent performance/cleanliness/size assessment
remain the critic's next step. The separate Linux sandbox and host-broker
contracts remain open; this correction does not claim their implementation.
