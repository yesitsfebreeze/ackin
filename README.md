# Cartridge

A base that runs programs as cartridges and wires them together with events.

A cartridge is a folder with a `cartridge.json` and an `init.lua`. The base
injects a `cartridge` global into every entry: the event system, streams and
registration. Everything a cartridge registers there is published in the base.
Code in another language ships as a Lua module the entry loads, or as a helper
program the entry talks to; neither imports anything from the base.

```lua
local fs = cartridge.load("fs")
cartridge.listen("tool.read", fs.read)
cartridge.listen("session.start", function(data) fs.watch(data.cwd) end)
```

Everything between cartridges is an event. `cartridge.json` declares the
events a cartridge defines (with a JSON Schema), listens to and needs. The base
refuses, before a cartridge starts, a subscription to an event nobody declares,
and refuses, before it is sent, a payload its schema rejects. Each cartridge
runs as its own sandboxed node; events go straight from sender to listener.

## Status

Early (`0.1.0`). Interfaces change without notice.

## Concepts

- **Cartridge**: `cartridge.json` declares `events` (name, description,
  schema), `listen` (listened), `needs` (must have a listener), `grant` (machine
  access) and `settings`. `init.lua` registers into the base.
- **Profile**: `.cartridge/init.lua` lists the cartridges a project runs and
  `.cartridge/config.lua` configures them. Installed cartridges not listed
  there are known but not started.
- **Events**: `emit`, `bail`, `parallel` and `gather` send an event to its
  listeners and differ in what they do with the answers. A tool is an event
  its owner listens to.
- **Streams**: a cartridge publishes on named channels and keeps a replay
  buffer; others subscribe.
- **Settings**: every tunable value is declared, then settled in layers: the
  declaration, `~/.cartridge/config.lua`, the project's `.cartridge/config.lua`.

## Build

Requires Rust 1.89+.

```sh
cargo build --release
cargo test --workspace
```

## Usage

```sh
cartridge setup                   # make this directory a project: choose cartridges, write .cartridge/init.lua
cartridge doctor                  # ask every composed cartridge whether it is healthy here
cartridge run <event> '<json>'    # start the profile, send, print the first answer, stop
cartridge daemon                  # start the profile and keep it running
cartridge call <event> '<json>'   # send on the running base, print the first answer
cartridge send <event> '<json>'   # send on the running base, print every answer
cartridge follow <channel>        # print a channel: lifecycle, or <cartridge>.<channel>
cartridge status                  # every cartridge and its state
cartridge reload [<cartridge>]    # reload the profile, or restart one cartridge
cartridge list                    # the profile and what each need binds to
cartridge settings [<id>]         # every setting and the file that settled it
cartridge verify [<cartridge>]    # send the contracts cartridges declare
cartridge help [<address>]        # the documentation of the base and every cartridge
```

`cartridge setup` offers the cartridge folders under the cartridge root (or
`--from <dir>`) and the repositories named in `~/.cartridge/catalog.json`
(`{"<name>": {"repository": "<url>", "description": "…"}}`), links or clones
the chosen ones under the root, writes `.cartridge/init.lua`, and sends each a
`setup` event it declares so it can ask what this project must decide.
`--with a,b` or `--yes` take cartridges without asking.

`cartridge mcp` and `cartridge launch` start the profile and hand this terminal
to the cartridges that listen to `mcp` and `proxy`.

## Writing a cartridge

See [docs/creating-cartridges.txt](docs/creating-cartridges.txt).

## Documentation

- [docs/architecture.txt](docs/architecture.txt): the base, lifecycle, nodes, sandbox
- [docs/transport.txt](docs/transport.txt): events, declarations, checks, the Lua global
- [docs/creating-cartridges.txt](docs/creating-cartridges.txt): Lua, Rust and helper programs
- [docs/writing-good-cartridges.txt](docs/writing-good-cartridges.txt): defining, listening, needing
- [docs/settings.txt](docs/settings.txt): declaring and settling configuration
- [docs/development.txt](docs/development.txt): building, testing, troubleshooting

## License

MIT
