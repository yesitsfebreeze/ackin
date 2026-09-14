# Cartridge

A host that runs programs as cartridges and wires them together.

A cartridge is a directory with a `cartridge.json` manifest and a Lua entry. The
entry is either the cartridge itself, run inside the host, or one line that
starts a program in any language:

```lua
return cartridge.process("my-program")
```

Every cartridge serves JSON-RPC on its own local socket. The host reads the
profile, binds each cartridge's needs to the cartridge providing them, starts
cartridges in dependency order inside the sandbox their manifest's `grant`
describes, and hands each one a directory of the sockets and tokens it may use.
Calls, events and streams then go directly between cartridges. The host
restarts a cartridge when its sources change and stops everything cleanly.

## Status

Early (`0.1.0`). Interfaces change without notice.

## Concepts

- **Cartridge**: declares in `cartridge.json` the keys it `provide`s, the keys
  it `needs`, the events it listens to (`on`), the machine access it asks for
  (`grant`) and its `settings`.
- **Profile**: `.cartridge/init.lua` lists the cartridges a project runs and
  `.cartridge/config.lua` configures them. Installed cartridges not listed
  there are known but not started.
- **Isolation**: a cartridge reaches another only through a key it declared in
  `needs`, or by sending an event another declared in `on`. The host enforces
  this with one token per edge.
- **Events**: `emit`, `bail`, `parallel` and `gather` send an event to its
  listeners and differ in what they do with the answers.
- **Streams**: a cartridge publishes on named channels and keeps a replay
  buffer; cartridges that need it subscribe.
- **Settings**: every tunable value is declared, then settled in layers: the
  declaration, `~/.cartridge/config.lua`, the project's `.cartridge/config.lua`.

## Build

Requires Rust 1.89+.

```sh
cargo build --release
cargo test --workspace
```

`src/transport/` is the protocol: a crate of its own, so cartridges written in
Rust depend on it without the host, and re-exported by the host as
`cartridge::transport`. `src/transport/wire.ts` is the same protocol for
TypeScript cartridges.

## Usage

```sh
cartridge run <key> '<json>'      # start the profile, call one key, stop
cartridge daemon                  # start the profile and keep it running
cartridge call <key> '<json>'     # call a key on the running host
cartridge send <event> '<json>'   # send an event to its listeners
cartridge follow <channel>        # print a channel: lifecycle, or <cartridge>.<channel>
cartridge status                  # every cartridge and its state
cartridge reload [<cartridge>]    # reload the profile, or restart one cartridge
cartridge list                    # the profile and what each need binds to
cartridge settings [<id>]         # every setting and the file that settled it
cartridge verify [<cartridge>]    # run the contracts cartridges declare
cartridge help [<address>]        # the documentation of the host and every cartridge
```

`cartridge mcp` and `cartridge launch` start the profile and hand this terminal
to the cartridges that provide `mcp` and `proxy`.

## Writing a cartridge

See [docs/creating-cartridges.txt](docs/creating-cartridges.txt).

## Documentation

- [docs/architecture.txt](docs/architecture.txt): the host, lifecycle, sockets, sandbox
- [docs/transport.txt](docs/transport.txt): the protocol every cartridge speaks
- [docs/creating-cartridges.txt](docs/creating-cartridges.txt): writing a cartridge
- [docs/writing-good-cartridges.txt](docs/writing-good-cartridges.txt): dependencies, events, isolation
- [docs/settings.txt](docs/settings.txt): declaring and settling configuration
- [docs/development.txt](docs/development.txt): building, testing, troubleshooting

## License

MIT
