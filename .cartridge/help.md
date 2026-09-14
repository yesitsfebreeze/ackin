# host

The runtime composes cartridges. Each cartridge is a directory with a
`cartridge.json` and an entry, installed by placing it under the cartridge
root and enabled by naming it in `.cartridge/init.lua`. The host resolves what
each one declares it needs, starts it, routes calls and events between them,
and disposes it. Nothing else in the system knows the list.

## Use

- `cartridge help`: every module, then its documents, then their sections — a
  picker on a terminal. `cartridge help <id>[/<file>[#<section>]]` opens or
  prints one node; `--json` gives it as data.
- `cartridge help <words>`: every line in every document holding the words,
  with the address that opens it.
- `cartridge list`: every cartridge and what each need binds to.
- `cartridge settings [<id>|<id>.<key>]`: every tunable value, its current value
  and the file that settled it. `--template` prints a config.lua to save.
- `cartridge run <key> '<json>'`: load the profile, call one service, dispose.
- `cartridge call <key> '<json>'`: the same against the host already running.
- `cartridge follow <channel>` and `cartridge tail`: watch events as they happen.
- `just build|test|check <id>`: build or verify one cartridge from `cartridge.ctg`.

## The rule every cartridge follows

A cartridge brings its own surface: its services, its tools, its settings, its
tests, its documentation and its help page. It registers into the host and
reaches another cartridge only through a key it declared in `needs` (a hard
dependency, so the host refuses to start it when the provider is absent) or by
listening to events. It never carries another cartridge's protocol for an
optional feature. Installing or removing a cartridge therefore changes nothing
outside it; the manual you are reading is rebuilt from what is present.

## Read next

- [Writing a cartridge](../docs/creating-cartridges.txt) — the two-file Lua
  example, the manifest fields, the SDK process, registration.
- [What makes a cartridge good](../docs/writing-good-modules.txt) — the
  isolation rule, events, declared dependencies, contract memos.
- [Architecture](../docs/architecture.txt) — repositories, the host, the wire,
  the announce, replacement, permissions.
- [Settings](../docs/settings.txt) — how a tunable value is declared and settled.
- [Development and operation](../docs/development.txt) — build, test, proxy,
  MCP, state.
- [Memos](../docs/memos.txt) — the record, its kinds, and the memo tool.
- [The guide for models](../llms.txt) — the same map in the order a model
  should read it.
