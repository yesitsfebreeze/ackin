# Repeating quality measurements

Run from any directory, with Python 3.9+ and the checkout's Rust toolchain:

```sh
python3 /Users/feb/dev/cartridge/.pearde/quality-tools/measure.py \
  /absolute/path/to/checkout /absolute/path/to/round-evidence --checks
```

The driver writes only the evidence directory, temporary benchmark profiles, and
a separate Cargo target under `/tmp`. The checkout argument can name another
worktree. Runtime sources and the manifest are never edited. `--locked` prevents
dependency resolution changes. Omit `--checks` when checks for that exact source
have already been collected separately. `--target-dir /tmp/unique-round-target`
can reuse a previous isolated build; never share it with concurrent checkouts.

The current driver targets the renamed `cartridge` package and binary and uses
`cartridge.process` in its generated Lua peer. Profiles are passed as absolute
temporary directories, so the runtime's renamed `.cartridge` default directory
does not affect the workload or touch personal profiles. The SDK example remains
`rpc_fixture`. Results identify this rename from historical `zirkle` explicitly.
Historical baseline evidence remains unchanged; use a new output directory for
each round. The driver refuses a directory already containing `results.json`,
including a partial earlier run. Do not run round 2 until the candidate has
been declared ready.

Keep the defaults across critique rounds: 320 warmups followed by 3,200 verified
requests per workload, concurrency 1 and 16, direct Lua and bidirectional Rust
SDK-to-Lua paths, and 30 daemon startup samples. Startup has zero excluded
warmups: first launch and subsequent launches are all retained. The payload is
1,143 bytes as compact JSON, including a 1 KiB string, an array, and typed fields.

`results.json` contains scoped source hashes, commit, native platform/toolchain,
commands, exit codes, physical and nonblank Rust lines split between production
and tests, dependencies, release image bytes and hash, daemon RSS, every raw
latency, median and nearest-rank p95. Separate stdout/stderr logs preserve check
failures. The driver also records its own SHA-256.

Source hashes cover every file under `src/`, including Lua and JSON fixtures,
every file under `.cargo/`, and existing `Cargo.toml`, `Cargo.lock`, `build.rs`,
`rustfmt.toml` and `.rustfmt.toml`. The directory trees are enumerated again at
the end: additions and deletions are detected alongside content modifications,
with start/end counts and explicit change lists. Symlink files are hashed by
content; symlink directories are not followed. This is complete for the declared
scope, not a hermetic build-input digest: external dependencies, system/user
Cargo settings, generated output and assets outside that scope are excluded.

Rust LOC counts physical and nonblank lines, including comments. Files under
`src/tests/**` count as tests/fixtures. Elsewhere, an exact `#[cfg(test)]` item
counts as test code from its attribute through its closing brace or semicolon,
including nested contents. A scanner masks ordinary/raw/character strings and
nested comments before matching boundaries, so braces inside fixtures do not
end a test module. Inclusive per-file test line ranges are recorded. Mixed files
appear in both groups, but each line is assigned once. Compound `cfg`/`cfg_attr`
conditions involving `test` are listed for manual classification; their boolean
logic is not inferred. This is an item-boundary scanner, not a Rust parser.

The historical baseline used a directory-only split of 6,830 production and
5,365 test physical lines. Round 1 identified another 44 inline test-only lines
(`turn.rs:257–294`, `cartridge.rs:177–180`, `lib.rs:16–17`). The comparable corrected
baseline is **6,786 production / 5,409 test physical lines**, or **6,430 / 5,185
nonblank lines**. Compare subsequent test-aware counts against those corrected
values without rewriting the raw baseline evidence.

The measurements are short local runtime workloads and include client overhead.
Concurrency 16 means batches of 16 requests over one persistent socket, with the
next batch submitted only after all 16 replies arrive. Throughput is measured
over the whole loop, while request latency starts after JSON request encoding
and ends after decoding the reply. The SDK fixture performs an echo callback
and two metadata callbacks for every request. The first startup can be much
slower than later launches; raw data preserves that outlier. No cache flushing,
cross-platform validation, external agent/tool workload, large-profile scaling,
resource leak assessment, or peak-throughput claim is implied. RSS covers the
daemon only. Review conclusions should account for these limits.
