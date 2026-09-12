---
complexity: 6
footprint:
  - src/main.rs
---

# spec03 — the listing names its problems, and a clash is a failure

`zirkle ledger` is the one view of the registry a person gets. It answers for
what would otherwise be silent: an unreadable document, a need nothing offers,
and — the answered fork — a need two entries of one scope offer. The exit code
carries the same three answers, so a clash found when you install is a non-zero
exit and not odd behaviour later.

## What already stands

`ledger_lines` in `src/main.rs` prints one line per installed cartridge in path
order — path, declared name when it differs from the folder, `provides`,
`exports`, `error: <why>` for an unreadable document — and under each entry one
line per need:

- `key <- path` for a `Bound::One`,
- `key <- ?` for a `Bound::None`,
- `key <- ambiguous (a, b)` for a `Bound::Clashed`, every offer named, in path
  order, the same offers on every run.

The count it returns is documents that would not read **plus** bindings that
clashed, so `Command::Ledger`'s existing `> 0` exit covers both. A need
nothing offers is deliberately not counted: listing a need nothing satisfies
is legal — the launch cares, not the registry — and `deps()` has always
printed `?` the same way.

Probe step 5b proves the clash end to end: `far` re-exporting `store.get` and
`other` providing it, a root-level ask prints
`store.get <- ambiguous (far, other)` and the run exits non-zero.

## What is left

Two CLI-level checks the probe never ran, both one fixture each:

- a need nothing offers — prints `?`, exits zero. Only asserted at the
  `resolve` level in `src/tests/ledger.rs`; never through the binary.
- an empty or nonexistent root — prints nothing, exits zero, because nothing
  installed is a state the host runs in rather than an error.

## Acceptance

- [x] A need nothing offers prints `?` and the ledger command exits zero, proven through the binary.
  - Probe gains the case: a cartridge whose only need no entry anywhere offers;
    assert its line reads `need <- ?` and that `run` exits 0. Without the
    clash-count change this pass it already passed; the box pins that counting
    clashes did not start counting unbound needs.
  - mason 2026-09-12: probe — `ok: the need is named with ?` and `ok: the exit
    is zero — the launch cares, not the registry` (step 7); full run `PROBE OK`.
- [x] An empty root prints nothing and exits zero, proven through the binary.
  - `zirkle --dir <empty> ledger` — output empty, exit 0. Matches
    `Ledger::scan`'s contract that a missing root is an empty ledger.
  - mason 2026-09-12: probe — `ok: an empty root prints nothing and exits
    zero` and `ok: so does a root that does not exist at all` (step 8).
- [x] An ambiguous binding still exits non-zero after the two checks above land, so neither new case weakened the clash.
  - Probe step 5b re-run, green.
  - mason 2026-09-12: probe step 9 — `ok: the clash is still named` and `ok:
    the exit is still non-zero — neither new case weakened the clash`; the run
    ends `PROBE OK` with 15 `ok:` lines, and `cargo test --offline` is `ok. 75
    passed; 0 failed` (repo gate).

## Verify and Proof

```sh
bash /Users/feb/dev/cartridge/.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-ledger && cargo test --offline
```