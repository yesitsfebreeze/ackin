# host

The base runs cartridges. A cartridge is a folder with a `cartridge.json` and an
`init.lua`, installed by placing it under the cartridge root and enabled by
naming it in `.cartridge/init.lua`. Everything between cartridges is an event,
declared with a schema and checked by the base; each cartridge runs as its own
sandboxed node with the `cartridge` global injected.

## Use

- `cartridge setup`: make this directory a project; `cartridge doctor`: ask each cartridge whether it is healthy.
- `cartridge help [<id>[/<file>[#<section>]]]`: the documentation of the host and every cartridge.
- `cartridge list`: the descriptor and whom each cartridge's sends reach, cycles marked.
- `cartridge settings [<id>|<id>.<key>]`: every setting and the file that settled it.
- `cartridge run <event> '<json>'`: start the descriptor, send one event, print the first answer, stop.
- `cartridge daemon`, then `cartridge status`, `call`, `send`, `follow`, `reload`, `stop`.
- `cartridge verify [<id>]`: run declared contracts.
- `cartridge call asp '{"op":"expand","entity":"file:src/a.rs"}'`: ask ASP, the base's own service, about an entity. ASP is the fabric with a better protocol: one world merged from every cartridge that declares an `asp` block. Ops: `types`, `expand`, `search`, `actions`, `act`.

## Read next

- [Architecture](../docs/architecture.txt)
- [Transport](../docs/transport.txt)
- [ASP](../docs/asp.txt)
- [Creating cartridges](../docs/creating-cartridges.txt)
- [Writing good cartridges](../docs/writing-good-cartridges.txt)
- [Settings](../docs/settings.txt)
- [Development](../docs/development.txt)
- [Guide for models](../llms.txt)
