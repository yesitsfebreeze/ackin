# probe — the-host-binary

The probe is the trim itself, left uncommitted in the repo root:
`Cargo.toml`, `src/` (three modules deleted, `memory.rs` split into
`reload.rs` and a deletion, four files de-orphaned), and the test tree moved
to `src/tests/` where `mod tests;` resolves.

`cargo-build-stall.log` is where `cargo build` stopped on both passes. It is
not a crate failure: `syspolicyd` sits at ~100% CPU and every newly created
Mach-O on the machine hangs in `_dyld_start`. Reproduce with `dyld-hang.sh` —
it should print `hi` and `exit=0`; while the daemon is pegged it exits 142.

Pass two re-tested three workarounds, all of which still hang: re-signing the
new binary with `codesign -f -s -`, running a plain `cp` of an existing
working binary (`/bin/echo`), and killing every stalled `cargo`/
`build-script-build` on the machine to release whatever XPC requests they
held. The peg outlives all three. Nothing short of restarting the daemon
clears it, and that needs a privilege this worker does not have.
