---
complexity: 8
footprint:
  - src/loader.rs
  - src/runtime.rs
  - src/fiber.rs
  - src/tests/reload.rs
---

# spec01 — the subtree transaction: replace one node by uid, the dependents following

`Host::replace_node(uid)` replaces one node of the running tree, addressed by
its fiber uid: a profile entry's uid dispatches through the existing
`replace_entry` (unchanged); a node composed inside another cartridge runs the
same transaction through the node's `rebuild` — prepare the old generation's
services, drain, take the gate write, compose the candidate with private
realms per provided key under the old generation's parent, isolate and
intercept, publish only an active candidate preserving the provided keys, and
dispose the old generation in dependency order. A rejected candidate
disposes, cancels the prepares, and finishes the transaction, so the next ask
can begin. What stood beside the node is untouched: only the swapped node's
binding is republished, and dependents hold stable services that follow it
while their own fibers never re-apply.

This unit stands in the lane (built, not committed) — `replace_node` and
`swap_node` in `loader.rs`, the `Rebuild` type on `Component`/`Fiber` in
`runtime.rs`, `Runtime::handle` and `Runtime::ctx_under` in `fiber.rs`, and
the composed-node rebuild in `context.rs`. The boxes below are what an
implementer re-verifies, and what a later worker must not break.

## Acceptance

- [x] `host.replace_node(uid)` on a composed node whose source changed on disk
      returns a new generation uid, and a service that reads through the node
      returns the rebuilt generation's value while the composing fiber keeps
      its uid
- [x] a candidate whose apply raises never publishes: the live generation
      still answers with the old version, and the transaction is finished —
      the next `replace_node` on the same node succeeds without a host
      restart
- [x] an entry owned by the profile and a node composed inside another
      cartridge both answer `replace_node`: the entry through `replace_entry`,
      the composed node through the rebuild, each swapping its own generation
- [x] a node that was never composed with a rebuild refuses `replace_node`
      with a named error instead of guessing at a rebuild, and an unknown uid
      is refused rather than ignored

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/hot-reload
cargo test --quiet --lib a_nested_node_is_swapped   # .pearde/.lanes/hot-reload/src/tests/reload.rs
```

Run output (2026-09-12; indented, NOT a fence — collect runs every fence
as a script and `test result: ok…` is not one):

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.06s

Boxes 1-3: `a_nested_node_is_swapped_and_its_dependent_follows` — new generation
uid returned (`assert_ne!(generation, inner)`), the reading service returns the
rebuilt generation's value (`{"version":2}`) while the composing fiber keeps its
uid, a raising candidate keeps the live generation and the next ask succeeds, and
the profile entry swaps through the entry path (`mark2`). Box 4: added
`an_uncomposed_node_and_an_unknown_uid_refuse_the_ask`:

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.00s

— a Rust-composed node without a rebuild refuses `replace_node` with
`this node was not composed to be replaceable`, and uid 4242 is refused with
`no running node carries uid 4242`.
