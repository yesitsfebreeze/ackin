---
complexity: 5
footprint:
  - src/context.rs
  - src/loader.rs
  - src/runtime.rs
  - src/tests/reload.rs
---

# spec02 — the handle that survives its own swap: a composed node addressed as a node

A uid names one generation, and `switch` publishes the replacement under a new
uid — so the `LuaFiber` handle a parent captured from `ctx:cartridge` goes
stale the first time the node it names is swapped, and a second
`fiber:reload()` fires at a retired uid and does nothing. This unit makes the
Lua surface address the node, not the generation: `fiber:reload()` re-finds
the live generation by what persists across the swap — the node's shared
`Reload` transaction, and for a composed node its `rebuild` — and fires the
same subtree transaction through `request_reload`, errors landing on the
report channel as the process-cartridge ask already does.

What already stands: `LuaFiber:reload()` fires `request_reload(uid)` and
`request_reload` falls through to `replace_node` for a uid no profile slot
owns (built, not committed). What is left: the node lookup by shared reload
identity, so the second reload after a swap still lands, and the test that
proves two consecutive reloads of one composed node both take effect.

## Acceptance

- [x] a composed node's Lua handle reloads its node twice in a row: both
      swaps publish (the second against the generation the first produced),
      and the reading service returns each generation's value in turn
- [x] a reload fired from a stale generation uid — one the tree already
      replaced — is refused with a named error rather than silently dropped,
      and the node still reloads from its live generation afterwards
- [x] `request_reload` still answers the process-cartridge ask (`{"reload":
      true}`) for a profile entry without change

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/hot-reload
cargo test --quiet --lib reload   # .pearde/.lanes/hot-reload/src/tests/reload.rs
```

Run output (2026-09-12; indented, NOT a fence — collect runs every fence
as a script):

        cargo test --quiet --lib a_composed_handle_reloads
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.07s

        cargo test --quiet --lib an_ask_at_a_retired_uid
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.07s

        cargo test --quiet --lib reload
    test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 87 filtered out; finished in 4.30s

What moved: `LuaFiber` now carries the node's shared `Reload` transaction beside
its generation handle (`src/context.rs`), `reload` fires through
`Host::request_reload_for` (`src/loader.rs`), which re-finds the live generation
by that transaction when the handle's own uid was retired by a swap — a uid
nothing carries is refused on the report channel with
`no live generation carries this node's reload transaction`, and a raw ask at a
retired uid keeps `swap_node`'s `no running node carries uid` refusal. The raw
entry ask `host.request_reload(uid)` — the call the `{"reload": true}` message
dispatches to at `src/cartridge.rs` — is unchanged and re-applies the profile
entry (`an_ask_at_a_retired_uid`'s last leg). `Reload::same` (`src/reload.rs`)
compares the transaction's shared allocations.
