---
kind: work
description: "Composition lives in one place that a fiber can own, and the host profile is its depth-zero instance"
status: open
level: 10
estimate: 2d
needs:
  - "[[a-nested-cartridge-is-a-first-class-entry]]"
---

# the-profile-is-the-root-composer

## Outcome

There is one implementation of composition — validate the entries, resolve their
needs, spawn, watch, bank, replace — and it does not care how deep it runs. The
host profile is that implementation at depth zero. A cartridge whose config
carries an entry list composes those entries by the same rules, with the same
lifecycle guarantees, and is itself a cartridge to whoever composes it. No second
entry-instantiation path remains in the tree.

## Check

- [ ] One test body, parameterized by depth, passes identically for a profile
      entry and for the same entry composed inside a cartridge: it reaches the
      service, replaces the entry on a source edit, and keeps the bank.
- [ ] A fixture cartridge whose config names two child entries serves a key that
      one child provides and the other consumes, with no key of either visible
      to the profile that composed the parent unless the parent provides it.
- [ ] Composing the default profile through the same composer keeps `just run`
      and `zirkle run tool.memo` behaving as before, proven by the existing
      profile, lifecycle and foreground tests passing unchanged.
- [ ] A grep-backed count in the result shows one entry-instantiation site, not
      two, and the old path is deleted rather than deprecated.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `Loader` owns composition end to end and only at the root:
`entries()` reads `.zirkle/<profile>/init.lua`, `expand` resolves globs
(`core/loader.rs:365`), `instantiate` spawns and banks each entry
(`core/loader.rs:677`), `reconcile` drives the set (`core/loader.rs:703`), and
`spawn` folds the entry's `isolate` list into the context (`core/loader.rs:671`).
A cartridge can already spawn a child (`core/context.rs:138`) but inherits none
of that. The two paths are the duplication this memo removes; per the repo's
no-legacy rule the flat one does not survive alongside the general one.

## Spec

Extract the entry set and its lifecycle into a composer value that takes a
directory, an entry list and a context, and returns handles. `Loader` keeps what
is genuinely the host's: the profile file, the file watcher, the build, the
socket. The composer is what a fiber can hold, so a cartridge that wants children
holds one.

The recursive case then needs no new concepts: a parent's `needs` are resolved in
its own scope, its children's keys default to its realm, and a child need declared
private (from [[a-cartridge-declares-what-it-needs]]) gets a fresh realm through
`Ctx::isolate` (`core/runtime.rs:545`), which descendants inherit
(`core/runtime.rs:184`). Sharing stays the default at every depth; otherwise a
tree of four cartridges that each need `router` gets four routers fighting over a
loopback port.

Deleting the old path is part of the work, not a follow-up: one composer, the
profile as its depth-zero call, and the fixtures that covered the flat path
rewritten against the parameterized test rather than kept beside it.
