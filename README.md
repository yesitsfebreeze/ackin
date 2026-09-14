# Cartridge

Cartridges on a socket: run in the back, handle the events.

Cartridge is a runtime that composes small, replaceable components — cartridges —
into one host. Each cartridge is a directory with a `cartridge.json` manifest and
an entry: either a Lua script or a Rust process speaking the SDK's JSON-lines
protocol. The host resolves what each cartridge declares it needs, starts it,
routes calls and events between cartridges, hot-reloads changed sources, and
disposes everything cleanly.

This repository is the host runtime (`src/`, with the command line under
`src/cli/`), its profiles (`.cartridge/`) and the guides (`docs/`). The host
logs through `tracing` (`CARTRIDGE_LOG` selects the level) and answers every
failure with one typed error. The cartridges themselves — agent, memo, memory, proxy, pty,
router, sessions, tools and others — live in sibling `*.ctg` repositories linked
under `builtin/`.

## Status

Early and under active development (`0.1.0`). Interfaces change without notice.
The guides describe this checkout; source contracts and tests are authoritative.

## Concepts

- **Cartridge** — a unit that owns one capability. It declares the services it
  `provide`s, the services it `needs`, the permissions it requests (`grant`), and
  the settings it contributes, all in `cartridge.json`.
- **Profile** — `.cartridge/<profile>/init.lua` selects which cartridges take
  part; `config.lua` beside it configures them. `default`, `mcp`, `proxy` and
  `tools` compose different uses.
- **Isolation** — a cartridge reaches another only through a key it declared in
  `needs` or by listening to an event. Installing or removing a cartridge changes
  nothing outside it. A Lua cartridge runs in a sandboxed interpreter (no `io`,
  no `require`, an `os` that only reads the clock and the environment); a process
  cartridge is confined by the `grant` its manifest declares.
- **Events** — `emit`, `bail`, `parallel` and `gather` dispatch an event to its
  listeners and differ only in what they do with the answers.
- **Settings** — every tunable value is a declared setting, settled in three
  layers: the declaration, `~/.cartridge/config.lua`, then the project's
  `.cartridge/config.lua`.
- **Memos** — typed Markdown records under `.cartridge/memos/` holding context,
  decisions, routines and system prompt contributions.

## Requirements

- Rust 1.89+ and Cargo
- [Just](https://github.com/casey/just)
- Bun and Python 3.9+ for the tooling, tests and smoke checks
- Git

## Getting started

The runtime builds on its own:

```sh
cargo build --release
```

The full composition expects the sibling cartridge repositories next to this one,
so that `builtin/<name>` resolves to `../../<name>.ctg`:

```
cartridge/
├── justfile          forwards recipes into cartridge.ctg
├── cartridge.ctg/    this repository
├── agent.ctg/
├── memo.ctg/
└── ...
```

Then, from `cartridge.ctg`:

```sh
just build            # runtime, Rust cartridges, memory, UI setup
just test             # full repository suites
cartridge help        # browse every module, document and section
```

## Usage

```sh
cartridge help [<address>]        # the manual; an interactive picker on a terminal
cartridge help <words>            # every documented line holding the words
cartridge help --json [<address>] # the same as JSON, for scripts and agents
cartridge list                    # cartridges of the profile and what each need binds to
cartridge settings [<id>]         # every tunable value and the file that settled it
cartridge settings --template     # every key as a config.lua ready to save
cartridge run <key> '<json>'      # load the profile, call one service, dispose
cartridge daemon                  # keep a composition running
cartridge call <key> '<json>'     # call a service on the running host
cartridge follow <channel>        # print events on a stream channel as they arrive
cartridge verify [<cartridge>]    # run the contracts cartridges declare
cartridge mcp                     # serve the profile's tools over MCP stdio
```

`cartridge help` is all the documentation in one place: `llms.txt`, every README,
`docs/`, the memos, each cartridge's help page and its `cartridge.json`
declarations. It is a tree of modules (`host` and each cartridge), their
documents, and each document's sections, and every node has an address:

```sh
cartridge help memo                             # the documents memo ships
cartridge help host/docs/memos.txt              # one document
cartridge help host/docs/memos.txt#record-format  # one section
cartridge help validated writes                 # every line holding both words
```

On a terminal each of these opens a picker at that node. Type to filter; from
two characters on, the list also shows matching lines from everything below.
Enter goes one level deeper or opens the document at that line, and Esc goes
back. Piped or run by an agent, the same commands print plain text, and `--json`
returns the same data as JSON.

Register the MCP server with a client, for example Claude Code:

```sh
claude mcp add cartridge -- cartridge mcp
```

Common `just` recipes:

| Recipe | Does |
| --- | --- |
| `just build [target]` | build everything, or one registered module |
| `just test [target]` | run all suites, or one module's |
| `just check [target]` | format check and strict Clippy |
| `just smoke [policy\|proxy\|mcp]` | isolated protocol checks, no model requests |
| `just run` / `just tui` | start Live and the terminal interface |
| `just proxy [port]` | authenticated model proxy on `127.0.0.1:4242` |
| `just describe <module>` | print a module's `cartridge.json` |

## Writing a cartridge

A minimal Lua cartridge is a `cartridge.json` and an `init.lua`; a Rust cartridge
implements `cartridge::sdk::Cartridge` and is started with
`cartridge.process("<binary>")`. See
[docs/creating-cartridges.txt](docs/creating-cartridges.txt) for the complete
example, the manifest fields and registration.

## Documentation

- [llms.txt](llms.txt) — entry point for agents working on this repository
- [docs/architecture.txt](docs/architecture.txt) — layout, host, wire, replacement, permissions
- [docs/creating-cartridges.txt](docs/creating-cartridges.txt) — authoring a cartridge
- [docs/writing-good-modules.txt](docs/writing-good-modules.txt) — events, dependencies, isolation
- [docs/development.txt](docs/development.txt) — build, test, proxy, MCP, troubleshooting
- [docs/settings.txt](docs/settings.txt) — declaring and settling configuration
- [docs/memos.txt](docs/memos.txt) — the memo record
- [.cartridge/README.md](.cartridge/README.md) — runtime profiles, diagnostics, streams

## License

MIT
