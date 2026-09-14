# The project profile

`init.lua` lists the cartridges this project runs; `config.lua` beside it
configures them, keyed by entry id. Entries are `{id, path, config?,
disabled?}`, with `path` relative to the cartridge root.

This repository is the base itself, not a composition, so its profile names no
cartridge and its `config.lua` pins nothing. What the directory holds is the
base's own: `settings.json`, `help.md`, and the tests under `tests/`.

## Settings

Every tunable value is declared: each cartridge in its `cartridge.json` under
`settings`, the base in `settings.json` here under `host`. Layers settle a key,
each laid over the last field by field:

| layer | file |
| --- | --- |
| declaration | the cartridge's `cartridge.json`, or `settings.json` for the base |
| this machine | `~/.cartridge/config.lua` (or `$CARTRIDGE_HOME/config.lua`) |
| this project | `.cartridge/config.lua` |
| the entry | `config = {...}` in `init.lua` |

```
cartridge settings              every key, its value, and the file that settled it
cartridge settings host         one module
cartridge settings --json       the same as data
cartridge settings --template   every key at its current value, as a config.lua
```

A non-zero exit from `cartridge settings` means a cartridge is configured with a
key nothing declares. `docs/settings.txt` is the full guide.

## Diagnostics

Set `CARTRIDGE_DIAGNOSTICS=stderr` or a file path to record diagnostics: the
host's own events and every line cartridges write to stderr, capped by
`host.diagnostics_max_bytes` with one rotated generation.
Diagnostics are written by a thread of their own, so a slow disk never stalls
the runtime. A record that finds the queue (`host.diagnostics_queue`) full is
dropped, and the next record written says how many were lost.
`CARTRIDGE_LOG` filters the host's own stderr (`warn` when unset).
