---
kind: routine
description: Run and inspect a built cartridge or profile through the runtime CLI, preserving caller arguments and exit status.
---

# Run the composed runtime

Build the selected owner first. These recipes use the runtime's normal CLI and
the compiled output under `CARGO_TARGET_DIR` (or `target/`). Profiles keep their
configuration in `.cartridge/`; stores, keys, sockets and journals stay ignored.
An invocation does not imply permission to repeat an uncertain mutation.

```just
set positional-arguments
binary := env_var_or_default("CARGO_TARGET_DIR", env_var("MEMO_OWNER_ROOT") / "target") / "debug/cartridge"

run *args:
    @"{{binary}}" run ui "$@"

daemon profile="default":
    @"{{binary}}" --profile "$1" daemon

invoke service payload="null" profile="default":
    @"{{binary}}" --profile "$3" run "$1" "$2"

call service payload="null" profile="default":
    @"{{binary}}" --profile "$3" call "$1" "$2"

status profile="default":
    @"{{binary}}" --profile "$1" status

tail profile="default":
    @"{{binary}}" --profile "$1" tail

ledger:
    @"{{binary}}" ledger

mcp:
    @"{{binary}}" --profile mcp mcp

live:
    @bun "$MEMO_OWNER_ROOT/../live.ctg/src/launch.ts" "{{binary}}"

launch agent *args:
    @"{{binary}}" launch "$@"

verify module="":
    @if [ -n "$1" ]; then "{{binary}}" verify "$1"; else "{{binary}}" --profile tools verify; fi

process module *args:
    #!/usr/bin/env bash
    set -euo pipefail
    module=${1%.ctg}; shift
    case "$module" in memo) name=memo_cartridge;; memory) name=memory_cartridge;; agent|docs|fs|gitfs|harness|mcp|memory-tool|proxy|pty|router|sessions|tools) name=$module;; *) echo "Not a Rust process cartridge: $module" >&2; exit 2;; esac
    exec "${CARGO_TARGET_DIR:-$MEMO_OWNER_ROOT/target}/debug/$name" "$@"

modules:
    #!/usr/bin/env bash
    set -euo pipefail
    for manifest in "$MEMO_OWNER_ROOT"/../*.ctg/cartridge.json; do
        bun -e 'const m=await Bun.file(process.argv[1]).json();console.log(m.name+"\t"+(m.description||""))' "$manifest"
    done
    echo 'runtime: native host; landscape: shared context library; memory/<package>: memory workspace member'

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
