---
kind: work
description: "A cartridge composes cartridges: needs resolve from declarations, a composed subtree is itself a cartridge, only the reached chain starts, and a folder can come from its repository"
status: open
level: 9
subwork:
  - "[[a-cartridge-declares-what-it-needs]]"
  - "[[a-nested-cartridge-is-a-first-class-entry]]"
  - "[[the-profile-is-the-root-composer]]"
  - "[[a-key-starts-its-provider-on-demand]]"
  - "[[a-cartridge-installs-from-its-source]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Composing cartridges inside cartridges, resolving what a cartridge needs, or deciding when a cartridge starts"]
---

# cartridges-compose-recursively

## Outcome

Composition is the same operation at every depth. A cartridge declares the keys
it provides and needs; the host resolves those needs to providers from
declarations rather than from a hand-wired profile; a cartridge that composes
other cartridges is itself a cartridge to whoever composes it; and a chain starts
when something reaches it, not when the host boots. A program is then a folder
set plus a small entry list, and extending the system is adding a folder — local,
or fetched from the repository its manifest names.

This does not replace [[cartridges-compose-live]], which owns live replacement of
a flat profile; it is what makes that model recursive. The three properties that
must not regress: one key has one active provider per realm, replacement stays a
transaction over the bank and the provided keys, and a cartridge that is not
enabled leaves no trace.

| Work | Estimate |
| --- | --- |
| [[a-cartridge-declares-what-it-needs]] | 1d |
| [[a-nested-cartridge-is-a-first-class-entry]] | 2d |
| [[the-profile-is-the-root-composer]] | 2d |
| [[a-key-starts-its-provider-on-demand]] | 2d |
| [[a-cartridge-installs-from-its-source]] | 2d |

## Check

- [ ] Every child is done with its recorded evidence.
- [ ] A profile naming one cartridge folder runs that cartridge's service with
      its chain resolved from declarations and no other entry started.
- [ ] `tool.memo` landscape graphs a nested composition as a tree: each child
      names its parent, its generation and its state, including
      declared-but-unstarted.
- [ ] `just run` and `zirkle mcp` behave as they do today, with the default
      profile composed through the recursive composer.
- [ ] `just check` and `just test` pass, and the flat composition path is gone
      from the tree rather than retained beside the general one.

## Approach

Observed 2026-09-12, reading `core/`. Three of the four pieces this needs already
exist: the per-cartridge contract (`cartridge.json` plus `provide`/`inject` keys
and the `selftest`/`integration` obligations, `core/loader.rs:73`), the
registration hooks (`ctx:provide`, `ctx:on`/`emit`/`bail`, `ctx:intercept`), and
the recursion primitive itself — `ctx:cartridge(path, config)` spawns a child
cartridge inside the caller's context (`core/context.rs:138`), and
`Ctx::isolate` gives a subtree a private realm for a key while unisolated keys
resolve to the parent's provider (`core/runtime.rs:545`, `:184`). Nothing in
`builtin/` calls it, so the primitive is present and unexercised.

What is missing is what the children own: declarations are learned by running the
cartridge rather than by reading it, children get none of the entry lifecycle,
composition exists only at depth zero, every enabled entry starts eagerly, and
`source` is metadata nothing acts on.

## Spec

Order is declarations, then depth, then demand, then retrieval — each child is
usable on its own, and each later one is cheap because the earlier one landed.
Two decisions constrain all of them.

Sharing is the default. A need is satisfied by the composer's provider unless it
is declared private; recursion without that default multiplies singletons that
must not be multiplied — one PTY, one router listener, one `sessions.dir`.

Cost follows reach. Every Rust cartridge is an OS process, so a deep tree is only
affordable if nothing starts until it is reached; demand activation is what makes
the recursive model cheaper than today's flat one rather than more expensive.
