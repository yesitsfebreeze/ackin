---
kind: decision
description: "Agent file writes land in a gitfs session branch in the project's own git, never on disk, until materialized or shipped"
status: accepted
date: "2026-09-12"
uses:
  - usage: "[[read-usage]]"
    when: ["Placing agent file I/O, choosing where session edits live, or wiring autogit-style ship flows"]
---

# gitfs-overlay-and-ship

## Decision

Agent file work in zirkle routes through a write-through overlay cartridge
(`builtin/gitfs`): every `tool.gitfs.write`/`edit` goes into a git branch
`refs/gitfs/<session>` in the project's own repository — the working tree and
the real index are never touched until `materialize`. Reads are session-first
(your-writes), then the worktree, then absence. Ship (`tool.ship`) is the
autogit flow on the current branch: it stages only the session's owned paths,
secrets-scans them, runs an optional LLM ship-or-hold gate that fails open,
commits with a `Shipped-by: gitfs` trailer, pushes with non-fast-forward
self-heal, and a trailer-gated `undo` rewinds remote-first under
`--force-with-lease`. No remote configured means the commit stays local.

## Why

Two prior decisions set the frame. `the-trunk-checkout-is-a-landing-pad` says a
shared checkout is a landing zone, so agent writes must not smear across it as
uncommitted dirt; `local-work-lanes` rejected whole-tree virtual materialization
because a lane is 99.92% build output. gitfs takes the middle path: agent source
edits accumulate as blobs under `refs/gitfs/<session>` in the repo object store
(no extra bare store, no disk writes, no `git add -A` on shared trunks), and the
overlay is only laid onto the real tree when a human materializes or ships it.

Ship diffing overlays owned paths onto the current trunk head rather than a
merge, so trunk movement between session start and ship can't present stale
full-tree snapshots. Secrets scanning ports autogit's hard-won exemptions:
template files (`.example/.sample/.template/.dist`) are skipped entirely,
placeholders are tested against the matched token only, and prompt-derived
subjects are never overridable even with `force_secrets`.

## Alternatives

- **Bare git-fs store** (separate repo): needs its own remotes and its own
  checkout machinery; ship/push must reach the same remote as the trunk, so the
  project's own git is the smaller surface.
- **Disk writes + watcher**: plan phase allowed a file watcher, but the tool
  layer is the agent's only file I/O path, so a watcher was deferred; the
  `snapshot` op covers shell-driven edits made outside the tool.

## Consequences

- `.zirkle/gitfs/` holds runtime state (private indexes, materialize ledgers) and
  stays ignored; only profiles and memos are tracked in `.zirkle/`.
- Programs that read disk (nvim, builds) don't see tool-layer writes until
  materialize — agents should use `tool.gitfs` for their own file work.
- `materialize` refuses to clobber on-disk files newer than the copy (mtime
  ledger), reporting conflicts by path; `force` overrides.
- The LLM ship gate runs only when a `gate_model` is configured and fails open,
  defaulting to a file-summary subject otherwise.
