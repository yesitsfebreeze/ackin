# The project profile

`init.lua` lists the cartridges this project runs; `config.lua` beside it
configures them, keyed by entry id. Entries are `{id, path, config?, inject?,
disabled?}`, with `path` relative to the cartridge root.

## Settings

Every tunable value is declared: each cartridge in its `cartridge.json` under
`settings`, the host in `settings.json` under `host`. Layers settle a key, each
laid over the last field by field:

| layer | file |
| --- | --- |
| declaration | the cartridge's `cartridge.json` |
| this machine | `~/.cartridge/config.lua` (or `$CARTRIDGE_HOME/config.lua`) |
| this project | `.cartridge/config.lua` |
| the entry | `config = {...}` in `init.lua` |

```
cartridge settings              every key, its value, and the file that settled it
cartridge settings agent        one cartridge
cartridge settings --json       the same as data
cartridge settings --template   every key at its current value, as a config.lua
```

A non-zero exit from `cartridge settings` means a cartridge is configured with a
key nothing declares. `docs/settings.txt` is the full guide.

## Diagnostics

Set `CARTRIDGE_DIAGNOSTICS=stderr` or a file path to record diagnostics: the
host's own events and every line cartridges write to stderr, capped by
`host.diagnostics_max_bytes` with one rotated generation.
`CARTRIDGE_LOG` filters the host's own stderr (`warn` when unset).
