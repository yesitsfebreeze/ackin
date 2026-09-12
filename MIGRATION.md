# Development home migration

The active source is `~/dev/cartridge/cartridge.ctg`. Sibling repositories own the
cartridges; their `cartridge.json` files describe their purpose and commands.
The parent now also has the explicitly requested forwarding Justfile.

## Information brought forward

- 472 Markdown memos plus the record's local lock were copied from sys into
  `.cartridge/memos/`. The lock stays ignored. A new development-home note records
  the migration. Active system/type/routine/usage/grammar/principle instructions
  use the cartridge name; historical notes and decisions retain their context.
- `CLAUDE.md` and `llms.txt` now describe the actual source layout and commands.
  The old versions, obsolete scripts and old runtime source remain recoverable
  through the preserved legacy Git repository.
- Local experiments were copied into ignored `scratch/`.
- Credentials, router state, host memory, sessions, MCP/proxy sessions and local
  dev state moved into `.cartridge/`. Old state paths link to these same files.
- The memory cartridge's nested memo Git repository moved into
  `../memory.ctg/.cartridge/memos`, preserving its history and uncommitted state
  and leaving its old path as a compatibility link.
- The memory CLI's `.memory` directory moved into `../memory.ctg/.memory`.
  Its database and ledger were moved by filesystem rename, not copied while live;
  its original path links to the moved directory.

Credentials, databases, sessions, logs, experiments and the legacy checkout stay
untracked. The migrated Markdown record and development guidance are versioned.
Local migration inventory: `validation/sys-migration.json`.

## Compatibility for running sessions

The user chose to preserve the running app and Claude sessions. Therefore
`~/dev/sys` is a temporary symlink to
`~/dev/cartridge/cartridge.ctg/.cartridge/legacy-sys`.
The old checkout's Git history and all six registered worktrees are intact and
were repaired after the move. No legacy process was stopped.

The legacy checkout and its build caches remain local while sessions depend on
them. Old memo edits made after the snapshot remain there; reconcile those edits
with the active record before removing it. New development and new sessions must
start from the new source repository, not the compatibility path.

After the old sessions close, the compatibility link and regenerable caches can
be retired. Preserve the legacy Git history, uncommitted worktree edits, local
experiments and any later memo changes first. This migration does not schedule
background deletion of user data or process termination.

## Verification

The parent commands were exercised against tools, runtime resolver tests, UI,
an internal memory crate, and the Lua policy. Invalid module names fail.
The proxy smoke starts the real proxy against a local router fixture and checks
HTTP authentication without model inference. MCP smoke checks initialize and
tools/list with isolated state. The actual parent `just mcp` additionally read
the migrated memo types through a real tools/call request.

The live development proxy was launched through `just proxy` at
`http://127.0.0.1:4242`; its private authentication key is in
`.cartridge/dev/proxy.key` unless supplied through `CARTRIDGE_PROXY_KEY`.
Validation output is kept locally under `validation/`.

The harness isolated `verify` requires configuration absent from its standalone
manifest. Its development guide runs the configured selftest and integration
services instead; both passed through the default profile.
