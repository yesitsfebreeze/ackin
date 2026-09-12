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

## Added pass three

`dyld-inode.sh` is the sharper reproduction. The verdict dyld waits on is
cached per **inode**, not per path: a hardlink of a working binary runs, a copy
of the same binary hangs. Healthy machine prints `0 0 0`; pegged prints
`0 0 142`. Choose a subject that ran successfully *before* the peg — a binary
created during it hangs from its own inode and looks like a counter-example.

`warm-cache-check.sh` answers, in one command, whether the
`CARGO_TARGET_DIR`-at-a-warm-tree workaround is worth trying. It is not, here:
all 31 proc-macro dylibs in `~/dev/sys/target` were themselves created during
the peg, so none carries a verdict. That route reaches the heavy leaf crates
(tokio, mlua, rustix, tempfile) and then blocks in `dlopen` — see
`[[260912-9edf]]`. Disabling the `kache` `RUSTC_WRAPPER` changes nothing.
