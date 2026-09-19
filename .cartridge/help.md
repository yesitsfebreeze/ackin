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
- `cartridge launch <agent>`: automatically set up a project without `.cartridge/init.lua`, then launch the agent. Cartridges come from the cartridge root or the nearest directory above the executable containing cartridge folders. The catalog is used only when no local cartridges are available. Existing project setup is preserved.
- `cartridge help [<id>[/<file>[#<section>]]]`: the documentation of the host and every cartridge.
- `cartridge list`: the descriptor and whom each cartridge's sends reach, cycles marked.
- `cartridge settings [<id>|<id>.<key>]`: every setting and the file that settled it.
- `cartridge run <event> '<json>'`: start the descriptor, send one event, print the first answer, stop.
- `cartridge daemon`, then `cartridge status`, `call`, `send`, `follow`, `reload`, `stop`.
- `cartridge verify [<id>]`: run declared contracts.
- `cartridge call asp '{"op":"expand","entity":"file:src/a.rs"}'`: ask ASP, the base's own service, about an entity. ASP is the fabric with a better protocol: one world merged from every cartridge that declares an `asp` block. Ops: `types`, `expand`, `search`, `actions`, `act`, `activity`.
- `cartridge call asp '{"op":"activity"}'` returns the last 1024 observed ASP uses and host dispatches with timestamps, entity IDs, observed node names and graph connections. It retains no request or response contents. Reads do not heat the graph; monitor expansions can set `observe:false`. History is host-local and starts a new generation after a restart. These are observed uses, not an inventory of every entity or every process's activity.

## Read next

Start the extensible world at `asp:root`; `event:<name>` expands an event declaration, and a trace provider adds `trace:root` with raw activity and compacted time tiers. A cartridge's `asp.roots` attaches its own entry points. Schemes, attributes and edges accept JSON Schemas that ASP checks when admitting contributions. Lua listeners implement both event behavior and ASP providers.

- [Architecture](../docs/architecture.txt)
- [Transport](../docs/transport.txt)
- [ASP](../docs/asp.txt)
- [Creating cartridges](../docs/creating-cartridges.txt)
- [Writing good cartridges](../docs/writing-good-cartridges.txt)
- [Settings](../docs/settings.txt)
- [Development](../docs/development.txt)
- [Guide for models](../llms.txt)

Agent tools expose `asp {op:"activity"}` through MCP as well as the native
`tool.asp` envelope. The answer retains up to 1024 observed uses in the current
host generation. Set `observe:false` on monitoring lookups to leave those use
counts unchanged; reading activity itself never records another use.

## Scope

`cartridge scope` opens the base-owned ASP browser. `cartridge scope --once`
prints a snapshot. The default launcher uses uv with pinned dependencies;
`--python /path/to/python` uses an existing Python 3.11+ environment.
The preview sits above the centered input and result list in a 50/50 split.
Ctrl-Space `vw` switches the preview between detail and waterfall. [Browser and extension contract](../ui/README.md) covers controls
and the `<owner>.scope` summary and `scope.detail.<scheme>` events.

The prompt-recall hook installed by setup queries ASP through the running host.
It attaches at most four bounded evidence descriptions, labels incomplete
providers and never treats retrieved content as instructions. It does not start
a second composition or call a separate memo context collector.

The center input accepts natural ASP filters such as `memo jev`. Ctrl-Space `aa`
switches to a real host agent conversation; `/agent`, `/ask memo jev`, and
`/list` offer the same flow. The list keeps at most 300 rows in a moving
100-item block window. Colors inherit the terminal palette, with only the
focused selection inverted.

Finder-style chains compose ASP and disk results: `Files src > Grep TODO`.
Tab completes or chains, Shift-Tab returns to the preceding stage, and Ctrl-Space `sm`
marks results. Ctrl-Space `ee` opens the preview editor; Ctrl-Space `es` saves through the owner
tool and its normal update events. Ctrl-Space `ex` uses `$VISUAL`/`$EDITOR` for the draft.
The default launcher installs Textual 8.2.8 and RapidFuzz 3.14.3 through uv.


Trace observations carry a stable Unix-millisecond timestamp in both the activity and writer envelope. Telemetry is redacted and bounded at queue admission and delivery: activities above 48 KiB retain operation, origin, event, timestamp and outcome summaries, with explicit truncation, original byte counts and SHA-256 digests of the redacted data. The complete payload is not retained in these summaries. This leaves room inside the writer's 64 KiB append limit. Permanent size or timestamp rejections are logged and counted as dropped before the next delivery, rather than blocking the queue with retries.

ASP search and expand accept optional `provider_timeout_ms` from 1 to 30000. This shared retrieval deadline retains completed provider evidence and explicitly marks timed-out sources unavailable. Omitting it preserves normal provider deadlines. The budget does not cancel work already running inside a provider.

The same limit applies at direct `trace` event dispatch, including user/assistant exchanges that bypass the telemetry queue. Oversized append envelopes also carry `writer_truncation` with the original redacted envelope byte count and digest.

Ctrl-Space `vr` refreshes the active finder chain. Switching away from the editor keeps its
unsaved draft until Scope exits; Ctrl-Space `ee` restores the draft and its original
revision guard. Agent-provided chains run through the same search engine.

Ctrl-Space opens a discoverable command tree: `ff` finds files, `fa` searches ASP,
`aa` opens the agent and `ee` edits the selected item. Type a sequence or search
by command name. Escape cancels; Backspace returns to the preceding group.
