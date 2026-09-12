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

- [ ] `cargo build --all-targets` prints `Finished` and exits 0
- [ ] `src/lib.rs` names no module that has no file, and no file under `src/` is unreachable from it
- [ ] `Cargo.toml` differs from `~/dev/sys/core/Cargo.toml` only in the four `path =` values
- [ ] `cargo build` emits no warning of any kind

## Verify and Proof

```sh
sh .pearde/prds/the-host-binary/probe/dyld-hang.sh
cargo build --all-targets 2>&1 | tail -20
diff <(sed 's|src/tests/|tests/|; s|"src/lib.rs"|"lib.rs"|; s|"src/main.rs"|"main.rs"|' Cargo.toml) ~/dev/sys/core/Cargo.toml
```
