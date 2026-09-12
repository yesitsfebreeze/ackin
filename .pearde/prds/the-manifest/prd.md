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

## Nesting is real, and a name travels only when it is passed on

The fork below is **answered**, and it is a premise of this PRD rather than a
question inside it. The user chose *hidden until passed on*: each piece names
what it offers outward, so names never clash and nesting is real.

Read as a contract on the document:

- A cartridge's `provide` keys are **private to that cartridge's own subtree by
  default.** A nested cartridge satisfies its parent's needs and nothing else;
  the graph outside the parent cannot see it and cannot bind to it.
- A parent that wants an inner name visible outward **says so explicitly** in
  its own manifest. That re-export is a declaration like any other — it is in
  the same document, and it is subject to the same reading by
  [[the-resolver]] and [[the-sandbox]].
- Therefore **two cartridges may provide the same key** without colliding, so
  long as neither subtree re-exports it into the other. A name is unique
  within a subtree, never across the whole graph.

This decides the shape of the data, which is why it was settled first. The
[[the-ledger]] is a **namespace of subtrees**, not a flat table: a key resolves
against the asking cartridge's subtree and walks outward, and an entry is
identified by its path, not by its bare name. [[the-resolver]]'s lookup walks
outward through subtrees rather than hitting a single map, and uninstalling a parent takes its
whole subtree with it — which is the same property that makes uninstalling mean
something, stated one level down.

The cost the user accepted is the wiring: an inner capability that genuinely
belongs to the outside has to be named twice, once where it is provided and
once where it is passed on. The cost they refused is a single global namespace
where the second cartridge to claim a name either loses or wins by accident.

## Questions

### Q1: What an inner cartridge offers the outside *(answered — see above)*

When one capability is built out of others, you are choosing whether the things
those inner pieces offer are visible to everything outside, or hidden until the
outer one chooses to pass them on. Visible everywhere means fewer steps and more
name clashes; hidden means real nesting and more wiring?

1. **Hidden until passed on** — each piece names what it offers outward, so names never clash and nesting is real. (recommended)
2. **Visible everywhere** — whatever an inner piece offers, the outer one offers too, with no wiring and possible name clashes.
3. **Visible unless hidden** — everything passes outward by default, and a piece can name the few things it keeps to itself.

<!-- for the board: cartridge.json `provide` keys; src/loader.rs inject-to-provider search and src/main.rs deps(). Flat vs subtree namespace decides the ledger's data structure and the resolver's lookup. Answer lands in the-manifest spec01, and is a premise for the-ledger and the-resolver. -->

## Answers

**Q1** *(answered 2026-09-12 15:39)* — Hidden until passed on — each piece names what it offers outward, so names never clash and nesting is real.
