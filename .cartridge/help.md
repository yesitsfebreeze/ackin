# host

The base runs cartridges. A cartridge is a folder with a `cartridge.json` and an
`init.lua`, installed by placing it under the cartridge root and enabled by
naming it in `.cartridge/init.lua`. Everything between cartridges is an event,
declared with a schema and checked by the base; each cartridge runs as its own
sandboxed node with the `cartridge` global injected.

A node keeps answering other events while a handler waits on another
cartridge, as long as the handler yields; a handler that cannot yield, such as
a native module's synchronous call, holds its node for the whole call.

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

ASP starts at `asp:root` and composes declared cartridge roots and event types.
Providers declare schemas for their nodes, attributes and edges. See
[ASP](../docs/asp.txt) for the protocol.

Agent tools expose `asp {op:"activity"}` through MCP as well as the native
`tool.asp` envelope. The answer retains up to 1024 observed uses in the current
host generation. Set `observe:false` on monitoring lookups to leave those use
counts unchanged; reading activity itself never records another use.

## Scope

`cartridge scope` opens the base-owned ASP browser. `cartridge scope --once`
prints a snapshot. The default launcher uses uv with pinned dependencies;
`--python /path/to/python` uses an existing Python 3.11+ environment.
The preview sits above the centered input and result list in a 50/50 split.
F4 switches the preview between detail and waterfall. [Browser and extension contract](../ui/README.md) covers controls
and the `<owner>.scope` summary and `scope.detail.<scheme>` events.

The center input accepts natural ASP filters such as `memo jev`. Ctrl-Space
switches to a real host agent conversation; `/agent`, `/ask memo jev`, and
`/list` offer the same flow. The list keeps at most 300 rows in a moving
100-item block window. Colors inherit the terminal palette, with only the
focused selection inverted.

Finder-style chains compose ASP and disk results: `Files src > Grep TODO`.
Tab completes or chains, Shift-Tab returns to the preceding stage, and Ctrl-T
marks results. Ctrl-E opens the preview editor; Ctrl-S saves through the owner
tool and its normal update events. F6 uses `$VISUAL`/`$EDITOR` for the draft.
The default launcher installs Textual 8.2.8 and RapidFuzz 3.14.3 through uv.
