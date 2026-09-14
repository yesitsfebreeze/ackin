# Runtime profiles

Profiles compose the sibling cartridge repositories through `builtin/` links.
`init.lua` composes; `config.lua` beside it configures. Configuration files
contain paths and defaults; credentials and runtime stores remain local.

## Settings

Every tunable value is a declared setting, not a constant. Each cartridge
declares its keys in its own `cartridge.json` under `settings`, with a type, a
default, bounds and one line of documentation; the host's own keys are declared
in `settings.json` beside the crate (read by `src/settings/host.rs`) and live
under `host`.

Three layers settle a key, each laid over the last field by field:

| layer | file |
| --- | --- |
| declaration | the cartridge's `cartridge.json` |
| this machine | `~/.cartridge/config.lua` (or `$CARTRIDGE_HOME/config.lua`) |
| this project | `.cartridge/config.lua` |

```
cartridge settings              every key, its value, and the file that settled it
cartridge settings agent        one cartridge
cartridge settings --json       the same as data
cartridge settings --template   every key at its current value, as a config.lua
```

The template changes nothing as printed: save it as either file and delete what
you do not want to pin. A non-zero exit from `cartridge settings` means a
cartridge is configured with a key nothing declares.

`config.lua` names what is this project's — where things are stored, which
addresses are bound, which model answers. Do not pin a limit there to restate
its default: the pin is what stops it following the declaration when the
declaration is raised. Defaults are generous on purpose. `docs/settings.txt` is
the full guide.

## Diagnostics

Runtime diagnostics are off by default. Set `CARTRIDGE_DIAGNOSTICS=stderr` or
set it to a file path to enable them; the cap on that file is
`host.diagnostics_max_bytes`, with one rotated generation, and
`CARTRIDGE_DIAGNOSTICS_MAX_BYTES` overrides it for one invocation.

What the host says about itself goes through `tracing`: `CARTRIDGE_LOG` is the
filter for stderr (`warn` when unset; `debug`, or `cartridge=trace,warn` to
raise only the host), and every host event is mirrored into the diagnostic
stream above when one is enabled, under the same trace id as the call it
belongs to. SDK cartridges read the same variable for their own stderr.

Automatic usage journals are opt-in through `record_usage = true` in each agent,
MCP, or proxy configuration. Session checkpoints and explicit memo/memory writes
remain durable.

## Streams

Event streams retain `host.stream_history_events` events and
`host.stream_history_bytes` per channel, whichever binds first. Runtime and SDK
subscriber queues are bounded at a full replay plus `host.subscriber_headroom`;
lagging consumers receive an error and must resynchronize from durable state.
Replay cursors older than retained history receive an explicit gap event.
