# Ackin

Ackin is a modular application runtime that composes independent components
through declared events, isolated processes, and managed lifecycles.

The name stems from `acknowledged integration` and `akin`.

```lua
local fs = cartridge.load("fs")
cartridge.listen("tool.read", fs.read)
cartridge.listen("session.start", function(data) fs.watch(data.cwd) end)
```

Everything between cartridges is an event. `cartridge.json` declares the
events a cartridge defines (with a JSON Schema), listens to and needs. The base
refuses, before a cartridge starts, a subscription to an event nobody declares,
and refuses a payload its schema rejects both when it is sent and when it
arrives. A cartridge sends only the events it defines or needs, over a token
minted for that one sender and listener. Each cartridge runs as its own
sandboxed node, with a Lua state, a memory cap and an instruction budget of
its own; events go straight from sender to listener.

## Status

Early (`0.1.0`). Interfaces change without notice.

## Concepts

- **Cartridge**: `cartridge.json` declares `events` (name, description,
  schema), `listen` (listened), `needs` (must have a listener; `name?` may be
  sent without waiting for its provider to start, which is how two cartridges
  that need each other both come up), `grant` (machine
  access and named environment variables) and `settings`. `init.lua` registers into the base. A grant is
  compiled into an operating-system policy the node starts inside —
  `sandbox-exec` on macOS, Landlock plus a seccomp socket filter on Linux; a
  platform that cannot confine refuses to start the cartridge. A granted
  program is allowed with its own installation, never with the person's home
  directory: a tool in `~/bin` brings that `bin` and nothing more.
- **Descriptor**: `.cartridge/init.lua` lists the cartridges a project runs and
  `.cartridge/config.lua` configures them. Installed cartridges not listed
  there are known but not started. Two ids that would share one node socket —
  differing only in case, or only in characters the socket name folds to `_` —
  are refused by name, so no cartridge unlinks another's live socket.
- **Trust**: a project runs only after `cartridge trust` records a SHA-256
  for each of its `*.lua` files and `cartridge.json` manifests under
  `~/.cartridge/trust`. A changed or unrecorded file is refused by name, so
  cloning a repository never runs its code. `shasum -a 256` reproduces every
  recorded hash. A node runs with a cleared environment — its own credential,
  `PATH`, `HOME` and locale only — so a cartridge cannot read the person's
  shell, and the host admin token never enters a sandbox.
- **Events**: `emit`, `bail` and `gather` send an event to its listeners and
  differ in what they do with the answers. Each listener's outcome is answered,
  declined, failed, timed out or unavailable, within the event's deadline.
  Handlers run as coroutines, so a node serves many events at once. A tool is
  an event its owner listens to.
- **Streams**: a cartridge publishes on named channels and keeps a replay
  buffer; others subscribe.
- **Settings**: every tunable value is declared, then settled in layers: the
  declaration, `~/.cartridge/config.lua`, the project's `.cartridge/config.lua`.
  A cartridge key no declaration names is listed as `undeclared` and makes
  `cartridge settings` exit non-zero; a stray `host` key is warned about and
  the host's declared defaults stand.
- **Sockets**: every socket of a run lives in a directory only this user can
  enter, under `$XDG_RUNTIME_DIR/cartridge` or `/tmp/cartridge-<uid>`. Each
  socket is owner-only from the moment it exists, both ends check the other's
  uid, and a name held by anything that is not this user's socket is refused
  rather than replaced.

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
cartridge trust [<dir>]           # record this project's Lua and manifests so this machine runs them
cartridge run <event> '<json>'    # send on the project's host, starting it when none answers
cartridge daemon                  # start the descriptor and keep it running
cartridge call <event> '<json>'   # send on the running base, print the first answer
cartridge send <event> '<json>'   # send on the running base, print every listener's outcome
cartridge follow <channel>        # print a channel: lifecycle, or <cartridge>.<channel>
cartridge status                  # every cartridge and its state
cartridge reload [<cartridge>]    # reload the descriptor, or restart one cartridge
cartridge list                    # the descriptor and whom each cartridge's sends reach, cycles marked
cartridge settings [<id>]         # every setting and the file that settled it
cartridge verify [<cartridge>]    # send the contracts cartridges declare
cartridge help [<address>]        # the documentation of the base and every cartridge
```

`cartridge setup` offers the cartridge folders under the cartridge root (or
`--from <dir>`) and the repositories named in `~/.cartridge/catalog.json`
(`{"<name>": {"repository": "<url>", "description": "…"}}`), links or clones
the chosen ones under the root, writes `.cartridge/init.lua`, and sends each a
`setup` event it declares so it can ask what this project must decide.
`--with a,b` or `--yes` take cartridges without asking. A cartridge's name is
the folder it lands in, so a name carrying a separator, `..` or `.cartridge` is
refused rather than placed. Setup records trust for
the descriptor it writes and each cartridge it links or clones, because it starts
them next. A folder or `config.lua` the tree already held stays untrusted until
you review it and run `cartridge trust`.

`cartridge mcp` and `cartridge launch` start the descriptor and hand this terminal
to the cartridges that listen to `mcp` and `proxy`. `cartridge launch claude --passthrough`
(or `-ps` after the agent) runs the agent against the proxy with no router in between:
its requests reach its own provider with its own login, enriched on the way.

Launching in a project without `.cartridge/init.lua` automatically runs setup
and continues. Setup takes the available cartridges from the cartridge root,
or from the nearest directory above the executable that contains cartridge
folders. If no local cartridges are available, it uses the configured catalog.
It creates the project descriptor and
records trust for the files it installs. Existing project descriptors and
configuration are preserved. Background startup creates `.cartridge` before
opening its daemon log.

`cartridge run|launch|daemon --yolo` asks once to trust the project's files and
let every cartridge and tool run without further prompts; Enter accepts, and the
trust is recorded so only changed files ask again.

Ctrl-C or a terminate stops a foreground command and every cartridge it
started, with the programs those cartridges spawned; while `launch`'s agent
runs, Ctrl-C is the agent's.

## Writing a cartridge

See [docs/creating-cartridges.txt](docs/creating-cartridges.txt).

## Documentation

- [docs/architecture.txt](docs/architecture.txt): the base, lifecycle, nodes, sandbox
- [docs/transport.txt](docs/transport.txt): events, declarations, checks, the Lua global
- [docs/asp.txt](docs/asp.txt): ASP composes events, cartridge roots and the activity trace in one extensible tree. Start with `cartridge call asp '{"op":"expand","entity":"asp:root","depth":2}'`. Cartridges declare roots and JSON Schemas, and implement their events and ASP providers in Lua.
- [docs/creating-cartridges.txt](docs/creating-cartridges.txt): Lua, Rust and helper programs
- [docs/writing-good-cartridges.txt](docs/writing-good-cartridges.txt): defining, listening, needing
- [docs/settings.txt](docs/settings.txt): declaring and settling configuration
- [docs/development.txt](docs/development.txt): building, testing, troubleshooting
- the source: every module under `src/` is a folder with a README saying
  why it exists; docs/architecture.txt maps them

## License

MIT

Agent tools expose `asp {op:"activity"}` through MCP as well as the native
`tool.asp` envelope. The answer retains up to 1024 observed uses in the current
host generation. Set `observe:false` on monitoring lookups to leave those use
counts unchanged; reading activity itself never records another use.

## ASP browser

`cartridge scope` opens the base-owned Textual browser: one searchable list, a
usage waterfall and a type-specific detail pane. The preview occupies the upper
half, the search input sits in the center, and results occupy the lower half.
Ctrl-Space `vw` switches between detail and waterfall. Plugins publish row summaries and detail documents.
The launcher uses uv to supply its pinned Python dependency, or accepts
`--python /path/to/python`. See [Scope](ui/README.md) for controls, installation
and the plugin contract.

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

Runtime ledger records now retain measured execution duration and explicit usage counts in a strict `metrics` object, together with known task/run/request identities. Structured diagnostics preserve their message, and oversize evidence keeps metrics and failure details. See [trace telemetry](src/trace/README.md) for the field contract, redaction rules and repeatable offline performance comparison. Missing measurements and verification results are never inferred from a successful request.
