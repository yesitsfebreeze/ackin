---
complexity: 5
footprint:
  - src/resolver.rs
  - src/lib.rs
  - src/tests/resolver.rs
  - src/tests/mod.rs
---

# spec01 — the walk is the launch plan, and a refusal is a refusal of the launch

The ask's resolution: `resolver::chain(&ledger, from, key)` walks the named
key's dependency chain to its far end, bottom-up, with the ledger's outward
lookup — a node's own needs bind inside its own subtree first and step outward,
so a provide key one subtree over is invisible however identical its name. The
walk returns the launch plan (far end first, the asked-for tool last) or a
`Refusal`: `Unbound` (a key nothing in scope offers — legal to list, not legal
to launch), `Ambiguous` (one scope offers the same key twice; the refusal names
every offer), `Cycle` (the walk closes on a node it is already walking
through). A diamond is not a cycle: the chain carries a shared provider once.
The cycle stack is inherited from `deps()`'s duty, not re-derived.

This unit stands in the lane (built, not committed); the boxes below are what
an implementer re-verifies, and what a later worker must not break.

## Acceptance

- [x] `resolver::chain` returns the whole chain bottom-up — a three-node ask
      yields `[far end, …, the tool that was asked for]`, and a nested
      provider inside the asking cartridge's own subtree answers before an
      outer lookalike that is not walked into
      > `a_chain_walks_to_its_far_end_before_the_tool_it_names`,
        `a_nested_provider_answers_before_any_outer_one` ok (12 passed)
- [x] a chain that closes on a node already on the walk stack refuses with
      `Refusal::Cycle`, naming the key and the node it resolves into again
      > `a_cycle_is_refused_not_walked` ok: `Refusal::Cycle { a.key, at: "a" }`
- [x] a key one scope offers twice refuses with `Refusal::Ambiguous` naming
      every offer by path; a key nothing offers refuses with
      `Refusal::Unbound` naming the key and where it was asked for
      > `a_clash_is_refused_naming_every_offer` (left, right),
        `an_unbound_key_refuses_the_launch` (absent.key at tool) ok
- [x] a diamond (two nodes needing one shared provider) resolves to a chain
      that carries the shared provider once
      > `a_diamond_launches_its_shared_provider_once`:
        `["shared", "left", "right", "top"]` ok

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-resolver
cargo test --lib tests::resolver::
```