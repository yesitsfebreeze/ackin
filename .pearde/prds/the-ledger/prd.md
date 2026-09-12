---
state: question
origin: requested
priority: 80
complexity: 0
blast-radius:
needs:
  - the-manifest
---

# The ledger

The registry the resolver reads: every cartridge installed on this machine, what
each one provides, what each one needs.

What exists today resolves within a single profile's hand-listed
`.zirkle/<profile>/init.lua`; `deps()` in `src/main.rs` and the loader's
inject-to-provider search walk only that list. A cartridge that is present but
unlisted does not exist.

Replace that: the host scans a cartridge root, reads each manifest, and derives
what is available from what is there. A profile becomes an override on top of the
ledger rather than the manifest of record.

Auto-registration is the requirement. Installing a cartridge is putting it where
the ledger looks. Uninstalling it is taking it away. Neither involves editing a
list, because a list that must be edited is a second place for the truth to live.

The namespacing decision in [[the-manifest]] is **settled**, and it fixes this
PRD's data structure before a spec is written: *hidden until passed on*. The
ledger is therefore a **namespace of subtrees, not a flat table.** An entry is
identified by its path from the root, never by its bare name; two cartridges may
provide the same key without colliding; a key resolves against the asking
cartridge's subtree and walks outward; and a parent that wants an inner key
visible outside re-exports it by name in its own manifest. Removing a parent
removes its whole subtree, which is what makes the auto-registration above
symmetric — putting a tree where the ledger looks installs everything in it,
taking it away uninstalls everything in it.

## Questions

### Q1: What a newly installed cartridge does

You are choosing what happens the moment a cartridge is dropped into the folder
the machine watches: it starts running on its own, or it becomes available and
waits to be asked for. Whichever you pick is what someone gets from a fresh
install, before they have configured anything?

1. **Available, not started** — a new cartridge is listed and can be asked for, and nothing of it runs until something needs it. (recommended)
2. **Starts as soon as it is installed** — dropping one in brings it up straight away, and stopping it means taking it away again.
3. **Waits for your yes the first time** — each new cartridge shows up as something pending your approval before it is ever allowed to run.

<!-- for the board: loader.rs Host::derived, the `disabled` default; flipping it costs one assertion in src/tests/bridge.rs (init.lua rewritten without a folder stops it) -->

### Q2: Two cartridges offering the same thing

Two cartridges sitting side by side can both offer the same capability, and
something asking for it by name then has two answers. Right now one of them wins
quietly and the other is never reached; you are choosing whether that silence is
acceptable, or whether it should stop and say so?

1. **Say it is ambiguous and stop** — the ask fails naming both offers, so you find out when you install rather than from odd behaviour later. (recommended)
2. **The nearest one wins, quietly** — the closest offer is taken and anything further out with the same name is ignored without a word.
3. **You name the winner once** — the clash is reported and you settle it yourself, and the machine honours that from then on.

<!-- for the board: ledger.rs Ledger::resolve, the same-scope candidate list; reproduced at step 5b of probe/the-ledger-is-the-tree.sh (`store.get <- far` with `other` also offering it) -->
