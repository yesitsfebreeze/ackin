# Cartridge development

Develop in `~/dev/cartridge/cartridge.ctg`. The parent owns submodule pointers and
a forwarding Justfile; runtime code, documentation and build orchestration live
here. Each sibling cartridge owns its implementation and `development.json`.

Read `.cartridge/memos/type/type.md` and `.cartridge/memos/type/system.md` for the
native record protocol. The active record was migrated from sys. Historical
notes retain their original context; the current layout and commands below
supersede old path examples. New session work belongs here, not in the legacy
compatibility checkout.

The runtime is under `src/`: runtime/fiber lifecycle, Lua composition, loader,
SDK processes, socket transport, and CLI. Installed cartridges are relative
links under `builtin/`; `.cartridge/<profile>/` supplies composition configuration.
The shared Rust cartridge workspace is `workspace/Cargo.toml`; the runtime and
memory retain their own workspaces. `scripts/workspace.py` shares build output.

Use `just modules` and `just describe <name>` to discover contracts and commands.
`just test <name> [filter]` selects one cartridge; `just test runtime resolver`
selects runtime module tests, and `just test memory/store` selects a memory crate.
`just check <name>` runs its format/lint gate. `just smoke` checks the real policy,
proxy and MCP transports with isolated local state; its proxy router is a fixture
so no configured model provider is contacted. Build first with `just build`.

`just proxy [port]` starts the authenticated development proxy (default 4242).
`just mcp` serves MCP over stdio; keep stdout free of diagnostics. Use
`just invoke <service> '<JSON>' [profile]` for a foreground service call or
`just call <service> '<JSON>' [profile]` for an already-running daemon.

Use the memo tool for validated record writes and `types` to discover the schema.
Enabled system memos feed the harness; cartridge-shipped records are read-only
contributions. Preserve private credentials, stores, sessions, and unrelated edits.
Rust uses tabs and denied warnings. Source profiles and manifests are reloadable.
Pearde's board remains in `.pearde/`; its uncompleted sandbox work is recorded there.

`~/dev/sys` temporarily links to `.cartridge/legacy-sys/` for existing sessions.
That checkout, its Git history and caches remain local while sessions use them.
See `MIGRATION.md` before retiring that link or reconciling later legacy memo edits.
