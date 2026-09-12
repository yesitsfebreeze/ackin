---
complexity: 8
footprint:
  - src/main.rs
  - src/cartridge.rs
---

# spec02 — the ask spawns the far end and exits; every link re-enters and execs

The launch verbs. `zirkle up <key>` resolves the chain once, checks every
node's program **before** anything spawns (a refusal belongs to the ask, not to
a link that already came up), spawns the far end detached, and exits — nothing
sits above the tree owning it. `zirkle enter <node> --rest <tail>` is one
re-entry: it re-reads the fresh ledger (a node no longer installed has no
launch), reads the document's `binary` — the field's first real consumer,
`cartridge::executable` went `pub` for it — and **execs** into that program in
place, so the dependency is the direct parent and no shim process exists. The
step rides in the environment: `ZIRKLE_NODE` (identity), `ZIRKLE_CHAIN` (node
paths still to launch), `ZIRKLE_ZIRKLE` (the binary to re-enter),
`ZIRKLE_ROOT` (ledger root), `ZIRKLE_BOTTOM` on the far end only (nothing
launched it that owns it) — and `enter` strips `ZIRKLE_BOTTOM` so it does not
leak into a dependent. The far end's stdio is null on all three: a detached
node must not inherit the asker's pipe, or a caller that captured stdout hangs
forever on a pipe nothing closes.

This unit stands in the lane; the boxes are the re-verification surface.

## Acceptance

- [x] `zirkle up <key>` leaves behind one process per node and exits, and
      `ps` shows each dependent a child of its dependency — the process tree
      and the dependency tree are the same tree, demonstrable, not asserted
      > `a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency`:
        `ppid(store) == db`, `ppid(tool) == store`; probe: `db ppid=1`,
        `store ppid=db`, `tool ppid=store`
- [x] a node whose document declares no `binary` refuses the ask naming the
      node, before anything of the chain spawns
      *(superseded by spec03: a no-binary cartridge is now **hosted** — the
      refusal moved to a declared `binary` that does not exist; this check
      was run and passed at this spec's state, output
      `test tests::resolver::a_chain_node_without_a_binary_is_nothing_to_launch ... ok`)*
- [x] a node uninstalled between the ask and the re-entry launches nothing:
      the re-entry re-reads the ledger and refuses, naming the node
      > `a_reentry_into_an_uninstalled_node_launches_nothing`: enter into a
        removed node → "`db` is no longer installed: nothing launches"
- [x] a cycle the ledger holds is refused by the ask, not only by the listing
      > `the_launch_refuses_a_cycle_the_ledger_holds` ok; probe:
        "refused: `a.key` resolves into `a` again: cycle"
- [x] the far end runs with `ZIRKLE_BOTTOM=1` and null stdio; a node reached
      by re-entry runs without the flag, and an ask whose stdout is captured
      returns when the hand-off is done
      > the ask ran under `.output()` (captured stdout) and returned with
        the whole tree up — the far end stayed alive without a pipe (bottom),
        and store/tool died with it, so they watched a pipe (no flag leaked)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-resolver
cargo test --lib tests::resolver::
sh /Users/feb/dev/cartridge/.pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh
```