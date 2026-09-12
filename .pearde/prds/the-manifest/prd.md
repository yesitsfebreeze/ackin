---
state: open
origin: requested
priority: 85
complexity: 0
blast-radius:
needs:
  - the-host-binary
---

# The manifest

What one cartridge declares about itself, and nothing beyond it: its name, what
it provides, what it needs, how it is entered. Today that is `cartridge.json`
plus a Lua entry returning `zirkle.process(...)`, described in `~/dev/sys/CLAUDE.md`
and implemented in `src/cartridge.rs` and `src/loader.rs`.

The change this PRD makes is that the same document is also the capability
request. What a cartridge declares it needs — filesystem paths, network, the
right to exec — is both what the resolver grants it and what the sandbox
confines it to. One document, read twice, by [[the-resolver]] and by
[[the-sandbox]]. There is no second grant file and no separate policy.

This is what makes uninstalling mean something: what a cartridge takes away when
it goes is exactly what it declared on the way in.

Whether a nested cartridge's `provide` keys are private by default or bubble up
to the parent graph is the open fork below. It shapes [[the-ledger]] and
[[the-resolver]] — a flat namespace and a namespace of subtrees are different
data structures — and should be settled before either.

## Questions

### Q1: What an inner cartridge offers the outside

When one capability is built out of others, you are choosing whether the things
those inner pieces offer are visible to everything outside, or hidden until the
outer one chooses to pass them on. Visible everywhere means fewer steps and more
name clashes; hidden means real nesting and more wiring?

1. **Hidden until passed on** — each piece names what it offers outward, so names never clash and nesting is real. (recommended)
2. **Visible everywhere** — whatever an inner piece offers, the outer one offers too, with no wiring and possible name clashes.
3. **Visible unless hidden** — everything passes outward by default, and a piece can name the few things it keeps to itself.

<!-- for the board: cartridge.json `provide` keys; src/loader.rs inject-to-provider search and src/main.rs deps(). Flat vs subtree namespace decides the ledger's data structure and the resolver's lookup. Answer lands in the-manifest spec01, and is a premise for the-ledger and the-resolver. -->
