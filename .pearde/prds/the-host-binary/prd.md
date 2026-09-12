---
state: specced
origin: requested
priority: 90
complexity: 32
blast-radius: high
workflow: probe-then-spec
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
