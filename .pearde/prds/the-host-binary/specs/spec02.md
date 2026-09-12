---
complexity: 14
footprint:
  - src/reload.rs
  - src/cartridge.rs
  - src/context.rs
  - src/loader.rs
  - src/lua.rs
  - src/runtime.rs
  - src/sdk.rs
  - src/service.rs
---

# spec02 — the swap guard survives the cut of the memory banks

The PRD cuts host-resident memory banks and, three paragraphs earlier, keeps
the lifecycle guard. Both lived in `src/memory.rs`. Q1 is answered *separate
them*: the guard stays, the stored data goes. This unit is that separation and
the retargeting of every call site it touches.

## What already stands

`src/memory.rs` is deleted. `src/reload.rs` (47 lines) holds what was the
reload transaction and nothing else: `gate` (an `RwLock` read-held by every
call and write-held across a switch), `epoch` (a `watch` channel a resumable
call waits on), `pending` (an `AtomicBool` claiming the transaction), and
`begin`/`pending`/`finish`. The type is `Reload`; it derives `Clone`, has a
hand-written `Default`, and carries no `serde` derive because it stores
nothing.

Cut with the bank: `Snapshot`, `Inner`, `read`, `update`, `fork`, `thaw`, and
their reach — `Sdk::memory` and `Sdk::checkpoint` in `sdk.rs`, the
`{"memory":"read"|"update"}` arm in `cartridge.rs`, `ctx:memory` and
`ctx:checkpoint` in `context.rs`, `Ctx::memory` in `runtime.rs`.

Two calls disappeared rather than being ported, and this is the only place the
change is more than a rename. With no private snapshot left to copy,
`fork()` degenerates to `Clone` and `thaw()` to a no-op, so in
`loader.rs::replace_entry` the `let next_memory = memory.fork()` /
`memory.thaw()` pair is now `component.reload = reload.clone()` and, on the
failure arm, nothing. The transaction it guards — `begin`, take the write
gate, stage the candidate, `switch`, `finish` — is unchanged step for step.

The field is renamed `memory` to `reload` on `runtime::Component`,
`runtime::Fiber`, `loader::Loaded` and `service::Service`, and the two
resident wrappings in `runtime.rs` (`Ctx::on`, `Ctx::provide`) and the
`service is being replaced` guard in `lua.rs::to_lua` follow it. No control
flow in `runtime.rs` or `fiber.rs` changed; the PRD forbids that and this is a
rename plus a deletion.

## What is left

Compile it. The rename was applied by exact-string match across eight files
with no compiler to check it, so a missed site is the expected failure mode
and the first box is what finds it.

## Acceptance

- [x] `sh /Users/feb/dev/cartridge/.pearde/prds/the-build-signal/probe/build-signal.sh /Users/feb/dev/cartridge/.pearde/.lanes/the-host-binary | tail -1` prints `signal=live` — **absolute paths, deliberately**: the lane worktree has no `.pearde/` of its own (the lane branch sits on `62fbd07`, which predates it), so the relative form this box used to carry does not resolve from the tree the work happens in. Run in the lane 2026-09-12 16:47: `dyld OK / cargo-build OK / zirkle-help OK / cargo-test OK (53 passed) / signal=live`, 15.3s wall. **Scope caveat:** the gate's stage 2 is plain `cargo build`, so `signal=live` on its own does not prove the examples and test targets compile — the next box is what covers those.
- [x] `cargo build --all-targets` exits 0 with the eight files above compiling — `Compiling zirkle v0.1.0` then `Finished ... in 3.36s`, rc=0 — a genuine codegen, forced by the real `sdk.rs` edit at 16:49:03 and not by a `touch` (see spec01 box 2: under `rustc-wrapper = "kache"` a touched build prints the same `Compiling` line in 0.43s and recompiles nothing). The rename applied by string match across eight files needed **no** compiler correction: zero diagnostics. Also in this unit, `sdk::on_reload` (the uncalled `pub fn` the last pass reported and could not test) was deleted, lines 57-65 of `src/sdk.rs`, and the build stayed green — the `reload` handler map it wrote to is still read by the dispatch arm at `sdk.rs:252`, so no cascade followed. **The deletion is proven by a test run that post-dates it:** the gate's `53 passed` came from 16:47, before the 16:49:03 `sdk.rs` edit, so `cargo test` was re-run in the lane at 17:08 against the edited source — `test result: ok. 53 passed; 0 failed; 0 ignored`, rc=0, 5.15s. Same 53, after the cut
- [x] `grep -rn "Snapshot\|checkpoint\|\.thaw()\|\.fork()" src` returns nothing — empty; `grep -rni "\bmemory\b\|development\|landscape" src` is also empty
- [x] `src/reload.rs` declares no field that stores cartridge data — only `gate`, `epoch` and `pending` (lines 12-14; 47 lines total, no `serde`)
- [x] `src/loader.rs` still calls `begin`, takes `gate().write_owned()`, and calls `finish` on both the success and the failure arm of `replace_entry` — `begin()` L821, `gate().write_owned()` L854, `finish()` L849 (prepare-failure), L881 (success), L896 (switch-failure fallthrough)
- [x] `src/fiber.rs` is untouched — `git diff 62fbd07 -- src/fiber.rs` is empty, and it was byte-identical to `~/dev/sys/core/fiber.rs` at 16:49 as well, that file being unmodified since 2026-09-11 19:08
- [x] `src/runtime.rs` was renamed, never redesigned — `git diff 62fbd07 -- src/runtime.rs` is empty, so nothing since the copy was frozen has touched it. The copy-to-baseline delta is **7 renamed lines, not the 8 this box first claimed** (L59, 71, 122, 150, 477, 579, 583) plus the `Ctx::memory` deletion at L387-396, verified at 16:49 against `~/dev/sys/core/runtime.rs` while that tree still stood still. **That tree is no longer a valid reference** — it was edited at 17:16:45 mid-pass and the copy was never taken from a commit, so there is nothing frozen upstream to point a box at. See `probe/provenance.md`.

## Verify and Proof

```sh
cargo build --all-targets 2>&1 | tail -20
grep -rn "Snapshot\|checkpoint\|\.thaw()\|\.fork()" src || echo "bank is gone"
git diff --exit-code 62fbd07 -- src/fiber.rs && echo 'fiber untouched since the freeze'
git diff --exit-code 62fbd07 -- src/runtime.rs && echo 'runtime untouched since the freeze'
grep -n "begin()\|write_owned\|finish()" src/loader.rs
```
