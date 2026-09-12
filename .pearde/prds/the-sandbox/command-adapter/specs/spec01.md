---
complexity: 6
footprint:
  - src/lib.rs
  - src/sandbox.rs
  - src/sandbox/
---

# spec01 — expose one confined command constructor

Export `sandbox::command` returning a standard process command compiled from
the cartridge's command, grant and root. `sandbox::spawn` wraps the constructor
with Tokio, piped stdio and kill-on-drop. Existing launch callers remain
unchanged; their migration belongs to launch-authority.

The implementation stands uncommitted, with the existing macOS profile compiler
and a Linux Unsupported stub. Three new tests cover empty input and actual OS
write denial through both entry points. Full verification now passes after recovery from the disk-full build failure.
All six acceptance checks are closed; source remains ready for board review.

## Acceptance

- [x] `cargo test --all-targets` succeeds after generated-artifact space is available; the first run was 114 passed and 13 fixture-build failures caused by disk exhaustion.
- [x] Empty command input returns InvalidInput before any process starts.
- [x] A synchronous macOS child cannot write outside an empty grant, and its shell completes to report denial.
- [x] An asynchronous macOS child has the identical write restriction and reports denial over stdout.
- [x] Profile tests preserve default deny, write paths, network semantics, executable resolution and script interpreter handling.
- [x] Linux preparation explicitly returns Unsupported until its backend exists, with no unconfined fallback.

## Verify and Proof

```sh
cargo test --all-targets
cargo test --lib sandbox
```

Inspect `src/sandbox/linux.rs` for explicit refusal. Linux policy enforcement
is not implemented in this unit. Runtime test fixtures live in temporary
directories, never in the board. After clearing only this lane's generated
incremental artifacts, the scoped sandbox check was rerun successfully.

Linux refusal inspected in `src/sandbox/linux.rs`: the complete function returns
`Err(std::io::Error::new(std::io::ErrorKind::Unsupported, ...))` and starts no
process. The cfg-selected branch in `sandbox::command` returns that result.

Full gate, 2026-09-12: `CARGO_INCREMENTAL=0 cargo test --all-targets` exited 0.
`test result: ok. 127 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 7.17s`. Binary and all five example test targets also exited 0
with zero tests. Compile completed in 4.45s.

Scoped gate: `CARGO_INCREMENTAL=0 cargo test --lib sandbox` exited 0:
`8 passed; 0 failed; 119 filtered out; finished in 0.46s`. This includes the
empty command test, synchronous and asynchronous live-wall tests, and all five
profile tests.
