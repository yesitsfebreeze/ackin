---
state: claimed
origin: requested
priority: 90
complexity: 32
blast-radius: high
workflow: probe-then-spec
claim: hostwright 2026-09-12 15:45
---

# The host binary

`src/` in this repo is a byte-identical copy of `~/dev/sys/core/*.rs` — the
zirkle core crate — with no `Cargo.toml`, no `tests/` and no `README.md`. It
does not build: `lib.rs` declares `mod tests` against a directory that was not
copied. Make it a crate of its own, and the smallest one that still holds the
mechanism.

Keep, because it is proven and it is the asset: `runtime.rs` and `fiber.rs`
(the dependency graph, revertible effects, the lifecycle guard — 1,053 lines),
`cartridge.rs` (the process wire), `sdk.rs`, `socket.rs`, `lua.rs`,
`context.rs`, `loader.rs`, `turn.rs`.

Cut, until a cartridge asks for it back: `development.rs` (cargo
rebuild-on-save, a `sys` workflow, not a property of this system),
`landscape.rs` (a read-only introspection projection built for that repo's memo
tool), `memory.rs` (host-resident memory banks — nothing here needs them yet).

Bring over `~/dev/sys/core/Cargo.toml` and `~/dev/sys/core/tests/`, minus what
the cut modules covered.

The language question is closed and does not reopen inside this PRD. Rust,
because the host embeds Lua (a C library), applies OS sandbox policy to its
children through syscalls, stays resident for days without leaking, and is the
one component in this design that is never rewritten on a whim — choosing a
scripting language would buy iteration speed on the file touched least.
Measured on this machine: the existing debug build answers `--help` in 0.9ms at
8.9MB RSS, against Bun 8.9ms/22.9MB, Deno 12.1ms, Python 15.6ms, Node 24.0ms.

Must not change: the semantics of the graph in `runtime.rs` and `fiber.rs`.
Trimming is deletion of whole modules, not redesign of what remains.

At the end there is a binary that builds, starts in about a millisecond, and
does nothing whatsoever on its own.

## Questions

### Q1: What happens to the cartridge memory banks

The part you asked to cut also holds the guard that lets a running cartridge be
swapped for a new one without dropping calls already in flight. Cutting the
whole part takes that guard with it, so the two have to be separated or kept
together?

1. **Separate them** — the swap guard stays, and only the stored data nothing has asked for yet goes. (recommended)
2. **Keep both** — nothing is removed here; the stored data stays until something asks for it back.
3. **Cut both** — stored data and live swapping both go, and reloading a cartridge means restarting the host.

<!-- for the board: src/memory.rs is Snapshot/read/update (the bank) fused with gate/epoch/pending/begin/finish/fork/thaw (the reload transaction). Bank callers: src/sdk.rs memory()/checkpoint(), src/cartridge.rs memory op, src/context.rs ctx:memory/ctx:checkpoint, src/runtime.rs Ctx::memory, tests/memory.rs. Gate callers: src/loader.rs replace_entry + request_reload, src/service.rs whole module, src/lua.rs to_lua guard, src/runtime.rs on()/provide() resident wrapping, src/fiber.rs Service downcasts. Answer lands in the-host-binary spec01. -->

## Answers

**Q1** *(answered 2026-09-12 15:17)* — Separate them — the swap guard stays, and only the stored data nothing has asked for yet goes.

## Board notes for the implementer *(written by the pass, 2026-09-12)*

**The build signal is still down, and it has been re-probed.** At 16:4x
`probe/dyld-hang.sh` printed `exit=142` — a freshly linked Mach-O still hangs
in `_dyld_start`. `ps` shows `/usr/libexec/syspolicyd` pid 498 at 98.6% CPU,
the same pid as yesterday. `cargo check --offline` was run to confirm what that
costs: it compiles roughly ten crates and then every process sits at 0% CPU
forever — the stall is the build scripts and proc-macro dylibs, which are
themselves freshly linked binaries. It was killed at 480s having made no
further progress.

The user was asked whether to restart the machine and **declined**. The remedy
they hold is restarting the daemon, not the machine — `sudo killall syspolicyd`
(launchd respawns it), with `sudo killall amfid` as its companion. That command
is theirs to run and it had not been run when this pass probed. **Re-run
`probe/dyld-hang.sh` yourself before trusting any compile.** If it prints `0`
the signal is back and every box is yours to tick honestly.

If it still prints `142`:

- **Carry on without the signal.** Do the editing work, tick every box that is
  a grep, a diff or a file-structure fact, and leave the compile boxes and the
  run boxes open.
- **Do not run `cargo build`, `cargo check` or `cargo test` speculatively** —
  they do not fail, they hang, and each one costs you ten minutes and a stuck
  process. One probe run is the whole test.
- **Say so plainly. Never write that something builds, passes or was verified
  when it was not.** A report claiming a compile it did not get is worse than
  an open box. Return `BLOCKED` naming the boxes the machine holds, with the
  probe's exact output, and list the work that *is* finished beneath it.

**This tree now has its own history.** The user settled it: *"Its own history —
this project starts tracking its own changes now, independent of whatever it
was copied out of."* `git init` ran at the root, `main` is the branch, and one
baseline commit (`62fbd07`, 100 files) holds everything the last two passes
left standing. `target/` and `.DS_Store` are ignored. Your lane is a real
branch off it and your work can be reverted.

**Standing direction from the user, verbatim:** *"This tree is a plain copy. We
need to remove everything that is not currently used by anything in the current
tree."* It is the reason this PRD exists and it is wider than the three cut
modules. The pass already checked the manifest against it: every dependency in
`Cargo.toml` is still reached from the kept modules — `notify` from
`loader.rs`'s watcher, `tempfile` from `src/tests/`, the rest throughout — so
the acceptance box that pins `Cargo.toml` to the original minus four `path =`
values stands unchanged, and no dependency is to be dropped. Where the
direction does still bite is inside the kept modules: an item that nothing in
*this* tree reaches is a candidate for deletion, not a thing to preserve
because the repo it was copied from used it. `warnings = "deny"` will find the
private ones for you the moment the compiler runs again; the `pub` ones it
never will, so note them in the report for the next spec rather than deleting
what you cannot compile against.
