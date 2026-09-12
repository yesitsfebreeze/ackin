---
state: done
origin: derived
priority: 99
complexity: 3
blast-radius: high
workflow: probe-then-spec
actual: 0.01h
---


# The build signal

Nothing on this board can be compiled, and until that changes no acceptance box
that names a compiler can be ticked honestly by anyone.

**What is wrong.** `/usr/libexec/syspolicyd` has been stuck at 100% CPU since
2026-09-11 — pid 498, 129 minutes of CPU time and climbing, unchanged across
three passes. While it is, dyld blocks forever on the code-signing verdict that
daemon is no longer answering, so **every freshly linked Mach-O hangs in
`_dyld_start` before reaching `main`.** The verdict is cached per inode, not per
path: a hardlink of a binary that already runs still runs, a byte-identical copy
of it hangs.

**Why it looks like a broken crate and is not one.** `rustc` itself still runs,
because its own verdict was cached long before the peg, so compiling a single
file looks healthy. What stalls is everything the build must *execute* —
`build.rs` scripts and proc-macro dylibs. `sample(1)` on a stalled `rustc` shows
it well past startup, inside `run_compiler`, blocked in `dlopen → mapSegments →
fcntl` on a proc-macro dylib. So `cargo build`, `cargo check` and `cargo test`
do not fail, they **hang at 0% CPU** after roughly ten crates, and each attempt
strands processes the next attempt inherits.

**What has been ruled out**, across three passes and eight attempts: waiting;
`xattr -c`; re-signing with `codesign --force`; disabling quarantine on the
output directory; a fresh terminal; a fresh toolchain; `cargo clean`; disabling
the `kache` `RUSTC_WRAPPER`; and reusing a warm `CARGO_TARGET_DIR` from
`~/dev/sys` — that last one gets further, reaching tokio, mlua, rustix and
tempfile, and then dies anyway, because all 31 proc-macro dylibs in that target
directory were themselves created during the peg and none pre-dates it.

**The consequence for requested work.** [[the-host-binary]] is `blocked` on this
and carries 8 open boxes — every box in spec03, and the `cargo build` box in
spec01 and spec02 — with the other 7 ticked on greps, diffs and file-structure
facts that need no compiler. Nine further PRDs sit behind the-host-binary:
[[the-manifest]], [[the-ledger]], [[the-resolver]], [[the-wire]],
[[the-sandbox]], [[the-telemetry-channels]], [[lua-interface]],
[[hot-reload]] and [[verify-one-cartridge]]. That is the whole board. Nothing
here ships, and nothing here is even *provable*, until this is cleared.

**What closes this PRD.** One command, and then one probe:

```sh
sudo killall syspolicyd    # amfid was never part of the fault
sh .pearde/prds/the-host-binary/probe/dyld-hang.sh    # must print exit=0
```

launchd respawns the daemon immediately; neither the machine nor the session
is lost. **That command needs `sudo`, so no agent on this board can run it** —
that is the entire reason this is a ticket rather than a task. When the probe
prints `exit=0` this PRD is `done`, [[the-host-binary]] unblocks straight back
to `specced`, and its remaining 8 boxes are one implementer's afternoon.

## The one path that does not need a privilege, and whether it holds

Before the fork below is put, there is one escape worth proving or killing:
**a Linux container compiles nothing through `syspolicyd`.** OrbStack is
installed at `/opt/homebrew/bin/orb` and `docker` resolves to its socket, but
the daemon is not running — `docker info` fails on a missing
`~/.orbstack/run/docker.sock`. Starting it needs no `sudo`.

What to find out, and it is a probe, not a design:

- Does the runtime start at all on this machine right now? Anything OrbStack
  launches is itself a Mach-O, so it may be caught by the same fault; a binary
  whose verdict was cached before the peg still runs, which is why this is a
  question and not an assumption.
- If it starts, does `rust:1.89` build this crate? `mlua` is `vendored`, so it
  needs a C toolchain in the image; `notify` and `tokio` are portable.
- **What the signal is worth if it works, stated plainly.** A Linux build
  proves the crate compiles, the module list resolves, the renames are right
  and `warnings = "deny"` is satisfied — which is every open box in spec01 and
  spec02. It does **not** prove the macOS binary: `--help` timing, RSS and
  anything touching OS sandbox policy are host-specific, and spec03's boxes
  stay open on a container pass. Say which boxes a container can close and
  which it cannot, and do not tick one it cannot.
- If the container path is dead, say why in one line and stop. Do not go
  looking for a fourth way around a daemon that needs `sudo` to restart.

Either outcome is worth the window: a working container is a compile signal the
board can have today without the user, and a dead one narrows the fork below to
two real options instead of three.

Must not change: nothing in the tree. This PRD writes no code. Its whole
deliverable is a working compiler this board can reach.

Reproductions, all runnable: `.pearde/prds/the-host-binary/probe/dyld-hang.sh`,
`probe/dyld-inode.sh`, `probe/warm-cache-check.sh`. The findings are on the
record as [[260912-7adb]], [[260912-9edf]] and the conclusion
[[a-pegged-syspolicyd-is-a-machine-fault-that-looks-exactly-li]].

## Questions

### Q1: How the machine gets its compiler back

Nothing on this board can be compiled, and the fix needs a level of access no
agent here has. A system daemon is stuck and every newly built program hangs
before it starts. Three ways out, and they cost very different things?

1. **Restart the two stuck daemons** — they come straight back, nothing else stops, and a twenty-second probe then says whether it worked. (recommended)
2. **Restart the machine** — the surest clear, at the cost of every session and every window currently open.
3. **Move the checks to another machine** — this one stays for editing only, and building and testing happen somewhere else from here on.

<!-- for the board: syspolicyd pid 498 at 100% since 2026-09-11; dyld verdict cached per inode; rustc blocks in dlopen->mapSegments->fcntl on proc-macro dylibs. Option 1 is two commands and is reversible by doing nothing. Option 3 is a new PRD of its own — a remote or container build path — and changes every spec's acceptance command. Answer lands here; the-host-binary unblocks to specced either way. -->

## History

**failed, retried 2026-09-12 16:45**

spec01: exit 1
dyld                   OK
cargo-build            OK
zirkle-help            OK
cargo-test             OK (53 passed)
signal=live
signal=live
  PID TTY           TIME CMD

## Report

spec01: exit 0
dyld                   OK
cargo-build            OK
zirkle-help            OK
cargo-test             OK (53 passed)
signal=live
signal=live
pid 498 gone
 %CPU COMM
  0.8 /usr/libexec/syspolicyd
 M .pearde/prds/the-host-binary/specs/spec01.md
 M .pearde/prds/the-host-binary/specs/spec02.md
 M .pearde/prds/the-host-binary/specs/spec03.md
 M .pearde/workflows/apply-the-answered-fork.md
 M .pearde/workflows/attempt-the-build.md
 M .pearde/workflows/probe-then-spec.md
 M .pearde/workflows/query-the-record-first.md
 M .pearde/workflows/read-the-contract.md
 M .pearde/workflows/record-what-the-build-learned.md
?? .pearde/prds/the-build-signal/
?? .pearde/wiki/sources/260912-ea92.md
