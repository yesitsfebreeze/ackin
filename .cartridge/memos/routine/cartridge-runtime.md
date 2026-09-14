---
kind: routine
description: Run and inspect the built cartridge runtime through its CLI, preserving caller arguments and exit status.
---

# Run the composed runtime

Build the selected owner first. These recipes use the runtime's normal CLI and
the compiled output under `CARGO_TARGET_DIR` (or `target/`). There is one user
profile: `.cartridge/init.lua` composes it and `.cartridge/config.lua` configures
it, and every command reads those two files. Stores, keys, sockets and journals
stay ignored. A host unlinks its socket when it stops; one that is killed
outright cannot, so a starting daemon collects what earlier kills stranded and
`just sweep` does the same on demand. An invocation does not imply permission to
repeat an uncertain mutation.

```just
set positional-arguments
binary := env_var_or_default("CARGO_TARGET_DIR", env_var("MEMO_OWNER_ROOT") / "target") / "debug/cartridge"

run *args:
    @bun "$MEMO_OWNER_ROOT/../live.ctg/src/launch.ts" "{{binary}}" "$@"

# The one launch command: the terminal interface and the voice service,
# attached to the same host, the same shell and the same conversation.
tui *args:
    @bun "$MEMO_OWNER_ROOT/../live.ctg/src/launch.ts" "{{binary}}" "$@"

# The terminal interface alone, with no voice service behind it.
solo *args:
    @"{{binary}}" run tui "$@"

daemon:
    @"{{binary}}" daemon

invoke service payload="null":
    @"{{binary}}" run "$1" "$2"

call service payload="null":
    @"{{binary}}" call "$1" "$2"

status:
    @"{{binary}}" status

tail:
    @"{{binary}}" tail

# Unlink the socket files no listener answers on. A runtime killed rather than
# stopped — a chain node taken down with its tree, a daemon that crashed —
# leaves its socket standing, and nothing else removes it. A daemon sweeps when
# it comes up; this is the same collection on demand.
sweep:
    @"{{binary}}" sweep

ledger:
    @"{{binary}}" ledger

mcp:
    @"{{binary}}" mcp

live:
    @bun "$MEMO_OWNER_ROOT/../live.ctg/src/launch.ts" "{{binary}}"

launch agent *args:
    @"{{binary}}" launch "$@"

verify module="":
    @if [ -n "$1" ]; then "{{binary}}" verify "$1"; else "{{binary}}" verify; fi

process module *args:
    #!/usr/bin/env bash
    set -euo pipefail
    module=${1%.ctg}; shift
    case "$module" in memo) name=memo_cartridge;; memory) name=memory_cartridge;; agent|docs|fs|gitfs|harness|mcp|proxy|pty|router|sessions|tools) name=$module;; *) echo "Not a Rust process cartridge: $module" >&2; exit 2;; esac
    exec "${CARGO_TARGET_DIR:-$MEMO_OWNER_ROOT/target}/debug/$name" "$@"

modules:
    #!/usr/bin/env bash
    set -euo pipefail
    for manifest in "$MEMO_OWNER_ROOT"/../*.ctg/cartridge.json; do
        bun -e 'const m=await Bun.file(process.argv[1]).json();console.log(m.name+"\t"+(m.description||""))' "$manifest"
    done
    echo 'runtime: native host; fabric: shared context library; memory/<package>: memory workspace member'

describe module:
    #!/usr/bin/env bash
    set -euo pipefail
    module=${1%.ctg}
    case "$module" in runtime|cartridge) directory=$MEMO_OWNER_ROOT;; *) directory="$(dirname "$MEMO_OWNER_ROOT")/$module.ctg";; esac
    if [ -f "$directory/cartridge.json" ]; then cat "$directory/cartridge.json"; else cat "$directory/.cartridge/docs/README.md"; fi
```

## Check

`just modules` lists existing manifests. `just process <owner> hello` preserves
the SDK protocol. Invalid modules fail before starting a process; failures and
cancellation retain the CLI's existing behavior.
