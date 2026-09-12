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

- [ ] `cargo build --all-targets` exits 0 with the eight files above compiling
- [ ] `grep -rn "Snapshot\|checkpoint\|\.thaw()\|\.fork()" src` returns nothing
- [ ] `src/reload.rs` declares no field that stores cartridge data — only `gate`, `epoch` and `pending`
- [ ] `src/loader.rs` still calls `begin`, takes `gate().write_owned()`, and calls `finish` on both the success and the failure arm of `replace_entry`
- [ ] `src/fiber.rs` is byte-identical to `~/dev/sys/core/fiber.rs`
- [ ] `src/runtime.rs` differs from `~/dev/sys/core/runtime.rs` in exactly 8 renamed lines and the 10-line `Ctx::memory` deletion, and in nothing else

## Verify and Proof

```sh
cargo build --all-targets 2>&1 | tail -20
grep -rn "Snapshot\|checkpoint\|\.thaw()\|\.fork()" src || echo "bank is gone"
diff src/fiber.rs ~/dev/sys/core/fiber.rs && echo 'fiber identical'
diff ~/dev/sys/core/runtime.rs src/runtime.rs
grep -n "begin()\|write_owned\|finish()" src/loader.rs
```
