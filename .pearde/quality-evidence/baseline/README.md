# Baseline evidence

Measured commit `92284ca5623bd95be073575ade146f6ea3e8706a` on 2026-09-12, before
the sandbox implementation. Runtime source and manifest had no uncommitted
diff and their hashes remained unchanged during measurement. The host was
macOS 26.2 arm64 with 10 logical CPUs, Rust/Cargo 1.98.0 and Python 3.9.6.

Command:

```sh
python3 .pearde/quality-tools/measure.py /Users/feb/dev/cartridge \
  .pearde/quality-evidence/baseline --checks
```

The private build target was `/private/tmp/zirkle-quality-target-hgo93yb4`.
The release build used default Cargo release settings, without extra RUSTFLAGS
or post-build stripping. The 1,143-byte JSON workload included a 1 KiB string,
an array, and typed fields. A persistent Unix socket carried all RPC calls.
Every reply was correlated and checked for exact expected data. SDK roundtrips
include a callback into host Lua and two host metadata calls.

| Workload | Measured requests | Warmups | Median | p95 | Completed calls/s |
|---|---:|---:|---:|---:|---:|
| Lua, concurrency 1 | 3,200 | 320 | 0.01875 ms | 0.02321 ms | 38,525 |
| SDK → Lua, concurrency 1 | 3,200 | 320 | 0.11833 ms | 0.13738 ms | 7,866 |
| Lua, concurrency 16 | 3,200 | 320 | 0.07996 ms | 0.14475 ms | 88,414 |
| SDK → Lua, concurrency 16 | 3,200 | 320 | 1.16785 ms | 1.36563 ms | 11,535 |

Daemon start to first verified SDK/Lua reply: 30 samples, zero excluded warmups,
median 8.817 ms, p95 15.167 ms, maximum 270.835 ms. The maximum was the first
launch. Subsequent launches mostly reused warm filesystem caches; cache state
was not controlled. Startup timing excludes fixture creation and socket-path
lookup and includes readiness polling in 1 ms intervals.

These are brief, local representative transport workloads, not a complete agent
session or sustained capacity test. Client request encoding is excluded from
individual latency but included in throughput; response reading and decoding
are included in latency. Concurrent workloads submit batches of 16 on one
connection. The Python client and concurrent host activity affect results.
External network tools, large profiles, reload scaling, queue growth, memory
leaks and other target platforms are unmeasured.

Size evidence:

- Production Rust: 16 files, 6,830 physical lines, 6,472 nonblank lines.
- Test/fixture Rust: 21 files, 5,365 physical lines, 5,143 nonblank lines.
- These line counts include comments and separate `src/tests/**` from runtime.
- Release arm64 Mach-O image: 5,660,224 bytes (about 5.40 MiB).
- Nine direct runtime dependencies; 78 unique resolved normal/build dependency
  packages for the host. The lockfile contains 97 packages including the root,
  development dependencies, and other platforms.
- Daemon RSS after RPC workloads: 11,536 KiB; this excludes the SDK child.

Validation:

- Release build passed.
- `cargo test --locked --all-targets`: **119 passed**, zero failed or ignored.
- `cargo fmt --all -- --check`: **failed**. The checkout uses tabs without a
  matching rustfmt configuration; the raw diff also records other formatting
  changes. No files were reformatted during measurement.
- `cargo clippy --locked --all-targets -- -D warnings`: **failed** at
  `src/tests/wire.rs:163`, `clippy::unnecessary_to_owned` for
  `host.call(&key.to_owned(), json!(null))`; clippy suggests passing `key`.

`results.json` retains raw measurements, exact per-file counts, source hashes,
environment, and command results. The accompanying stdout/stderr logs contain
the full check output. No quality scores are assigned by this measurement pass.
