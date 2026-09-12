---
kind: work
description: "builtin/memory is tracked source like every other cartridge: the vendored duplicate, the adapter patch and the bootstrap recipes go"
status: open
level: 10
estimate: 1h
uses:
  - usage: "[[read-usage]]"
    when: ["bootstrapping a fresh checkout, touching the memory workspace, or deciding how a cartridge's source enters the tree"]
---

# the-memory-workspace-is-tracked-not-patched

## Outcome

`builtin/memory` is ordinary tracked source: the upstream tree and our cartridge
adapter as files a reader can open, in this repository, with no bootstrap step
before a fresh clone can build it. Nothing applies a patch to produce a
cartridge, and no cartridge's source is stored twice.

What goes: `vendor/kern-58850707f37f7cd10939b742c4c9898e84552193/` (406 tracked
files, 6.0M — a second copy of the same tree), `builtin/memory-cartridge.patch`,
the `memory-bootstrap` and `memory-present` recipes, and the `/builtin/memory`
line in `.gitignore`. The `memory-build`, `memory-check` and `memory-test`
recipes stay, minus their `memory-present` dependency.

Scope is this one cartridge's source entering the tree. Removing the memory
cartridge itself, or its profile entries, is not part of this outcome.

## Check

- [ ] `git ls-files builtin/memory | head` lists the workspace's own files, and
      `git ls-files vendor` is empty.
- [ ] A fresh clone builds it with no bootstrap step: `git clone` into a temp
      directory, then `just memory-build` succeeds without `just memory-bootstrap`.
- [ ] `just memory-check` and `just memory-test` pass, kern's
      `tracked_artifacts` gate included — tracked in this repository satisfies
      `git ls-files` from that subdirectory, so the workspace no longer needs a
      `.git` of its own.
- [ ] `rg -n 'memory-cartridge\.patch|memory-bootstrap|memory-present|vendor/kern' --glob '!*.md'`
      returns nothing outside this memo's own history.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12 while auditing whether each cartridge is only what it needs
to be. The mechanism in `justfile:63`: copy `vendor/kern-<pin>/` into
`builtin/memory/`, `git init` it, `git apply builtin/memory-cartridge.patch`,
then auto-commit as "momo bootstrap". The patch is 771 added lines across 14
files, and roughly 660 of them are **our own code** — `cartridge.json`,
`init.lua`, `src/cartridge.rs`, `CARTRIDGE.md`, `tests/cartridge.rs` — held as a
diff that has to be applied before anyone can read it. The remaining ~130 lines
edit upstream files (`src/commands/`, `src/store/src/registry.rs`,
`tests/declared_dependencies.rs`); those become ordinary edits to tracked files,
and the upstream pin they were made against is recorded here and in this memo's
commit rather than in a directory name.

The apparatus exists to keep the workspace independently versioned against
`https://github.com/yesitsfebreeze/kern`, whose history was rewritten: the pin
is no longer reachable from the remote, which is why the tree is vendored in the
first place. Independence from an unreachable remote buys nothing and costs a
bootstrap step, a duplicate tree, and our source in diff form. The rename to
zirkle showed the cost directly — the bootstrapped copy still said
`cile.process` and `cile = { path = ... }` after the sweep, because a tree-wide
rename cannot reach a directory this repository ignores.

One-time care when landing: `builtin/memory` currently holds its own `.git`, and
lanes have held it as a symlink to the trunk's copy. Drop the symlink in any
worktree, and remove the nested `.git` as part of the import commit. Its
`target/` is large (70G on the observed checkout) and stays ignored.

The general mechanism is separate work: a cartridge fetched and pinned from the
repository its manifest names is [[a-cartridge-installs-from-its-source]], under
[[cartridges-compose-recursively]]. That is what this hack is a hand-rolled,
single-purpose version of — but it does not block this cleanup, and this cleanup
should not wait for it: a vendored tree that no remote can serve is not an
install case, it is source we own.
