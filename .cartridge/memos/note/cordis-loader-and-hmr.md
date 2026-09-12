---
kind: note
description: "Cordis's loader reconciles declarative entries by id and its HMR swaps fibers per module; the transactional rollback covers only the import and cache phase, nothing after an old instance is unloaded"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking how Cordis reloads configuration or code without restarting, and where that can fail half-way"]
    tags: [research]
---

# cordis-loader-and-hmr

Paper §5.2: an orchestrator writes the composition as entries (id, component,
config, realms, disabled) and the loader turns entry changes into fiber
operations in both directions. §5.2.2, Algorithms 8–10: HMR classifies
changed modules to a fixed point over the import graph (accepted when one
import is accepted, declined when all are, cycles decline), finds stale
entries whose dependency tree meets an accepted module, then reloads them
with cache backup so an import failure restores caches and old fibers.

Source ([[cordis-repo]]): groups match entries by id, update and create
concurrently, and log a failed update instead of rolling the tree back
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/loader/src/config/group.ts#L19-L68). File-backed config in
`packages/include` journals runtime edits and lets the file win on conflict;
writes go through temp file and rename, not an interprocess compare-and-swap
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/include/src/journal.ts#L187-L241). HMR validates new imports
before disposing anything (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/hmr/src/index.ts#L295-L407), then per
fiber awaits the old instance's drain and creates the replacement
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/hmr/src/index.ts#L409-L467). A replacement that fails to
initialize leaves a failed fiber; the old one is not restored. Module
top-level side effects are not undone by restoring the cache. Source HMR needs
Node loader internals and is disabled without them.

What would change this: a rollback branch after unload upstream. Model
behind it: [[cordis-revertible-effects]].
