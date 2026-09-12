---
complexity: 8
footprint:
  - Cargo.toml
  - src/lib.rs
  - src/main.rs
---

# spec01 — the copy becomes a crate that cargo can see

`src/` arrived as loose `.rs` files with no manifest and a `mod tests;`
declaration pointing at a directory that was never copied. This unit makes the
copy a standalone crate: a manifest of its own, a module list with the three
cut modules gone, and the test tree in the one place `mod tests;` resolves.

## What already stands

`Cargo.toml` is copied from `~/dev/sys/core/Cargo.toml` with `[lib] path`,
`[[bin]] path` and both `[[example]] path` entries repointed under `src/`.
Everything else in it is byte-identical to the original, including
`autotests = false`, `default-run = "zirkle"` and `warnings = "deny"`.

`src/lib.rs` lists eleven modules. `development`, `landscape` and `memory` are
gone; `reload` is in their place. `src/main.rs` is unchanged from the copy.

The test tree was moved from `tests/` to `src/tests/`. In `~/dev/sys/core` the
tests sat beside `lib.rs` and were reached as a private module, which is what
`autotests = false` encodes; putting them under `src/` restores that and is
what makes `#[cfg(test)] mod tests;` resolve. A root `tests/` directory would
read as integration tests, which these are not — they use `crate::lua::Host`.

## What is left

Run the build. No compile signal has been obtained on this machine yet (see
`## The build could not be run` in the PRD's report): every newly linked
Mach-O hangs in dyld while `syspolicyd` is pegged, so `cargo build` stalls
after ~10 crates with every process at 0% CPU. Confirm the machine is healthy
first with `probe/dyld-hang.sh` — it prints `exit=0` when it is — and only
then trust a build result. Fix whatever the compiler reports; with
`warnings = "deny"` an orphaned import or a dead function is a failure, not a
warning.

## Acceptance

- [x] `sh /Users/feb/dev/cartridge/.pearde/prds/the-build-signal/probe/build-signal.sh /Users/feb/dev/cartridge/.pearde/.lanes/the-host-binary | tail -1` prints `signal=live` — **absolute paths, deliberately**: the lane worktree has no `.pearde/` of its own (the lane branch sits on `62fbd07`, which predates it), so the relative form this box used to carry does not resolve from the tree the work happens in. Run in the lane 2026-09-12 16:47: `dyld OK / cargo-build OK / zirkle-help OK / cargo-test OK (53 passed) / signal=live`, 15.3s wall. **Scope caveat:** the gate's stage 2 is plain `cargo build`, so `signal=live` on its own does not prove the examples and test targets compile — the next box is what covers those.
- [x] `cargo build --all-targets` prints `Finished` and exits 0 — `Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.35s`, rc=0, and the compile was forced by a real one-byte source change, not by a `touch`. **The distinction is the point, and an earlier draft of this box had it backwards.** `rustc-wrapper = "kache"` is set in `~/.cargo/config.toml`, so on this host the two look identical and are not: `touch src/lib.rs` then build prints `Compiling zirkle v0.1.0` and `Finished in 0.43s` — a cache hit that recompiles nothing — while appending one newline to the same file prints the same `Compiling` line and takes **3.35s**, a genuine codegen. Both were run back to back to show it. The 3.35s figure is the one that proves the crate compiles; the tree was restored with `git checkout src/lib.rs` afterwards and `git status` is clean.
- [x] `src/lib.rs` names no module that has no file, and no file under `src/` is unreachable from it — 11 modules listed, 11 files present; only `lib.rs` (the lib root) and `main.rs` (the `[[bin]]`) are not modules
- [x] `Cargo.toml` is the manifest the copy was frozen with — `git diff 62fbd07 -- Cargo.toml` is empty. It differs from `~/dev/sys/core/Cargo.toml` only in the four `path =` values, normalised diff empty and raw diff exactly lines 12, 16, 21, 26, checked at 16:49; that upstream tree is no longer a valid reference and the box no longer reads it (`probe/provenance.md`)
- [x] `cargo build` emits no warning of any kind — the 3.35s genuine-codegen build above printed `Compiling zirkle` then `Finished` and nothing between. `[lints.rust] warnings = "deny"` (Cargo.toml L41-42) makes any warning a non-zero exit, and the exit was 0. Note this box is only meaningful on the real codegen: a cache-hit build re-emits nothing and so could never have shown a warning.

## Verify and Proof

```sh
sh /Users/feb/dev/cartridge/.pearde/prds/the-host-binary/probe/dyld-hang.sh
cargo build --all-targets 2>&1 | tail -20
git diff --exit-code 62fbd07 -- Cargo.toml && echo 'manifest untouched since the freeze'
```
