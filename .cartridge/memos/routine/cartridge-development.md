---
kind: routine
description: Build, check, and test the composed cartridges through their existing Cargo and Bun owners, with no Python dependency.
---

# Develop a cartridge

Run `just build <owner>`, `just check <owner>`, or `just test <owner>` from the
composed root or runtime. `all` covers the runtime, shared Rust workspace,
memory, UI, live, authentication, planning and workspace clients. Additional arguments retain their shell boundaries.
Memory owns its independent Cargo workspace. Tests and fixtures live in each
owner's `.cartridge/tests/`; outputs are ignored empirical state.

This memo is the command implementation. The small `memo-run` adapter extracts
its single just block; it does not implement another build system. A failed
underlying command returns its failure. Model evaluations remain explicit opt-in
operations in their own memos.

```just
set positional-arguments
runtime := env_var("MEMO_OWNER_ROOT")

build module="all" *args:
    @just --justfile "$MEMO_JUSTFILE" --working-directory "$MEMO_OWNER_ROOT" _cargo build "$@"
    @just --justfile "$MEMO_JUSTFILE" --working-directory "$MEMO_OWNER_ROOT" links

check module="all" *args:
    @just --justfile "$MEMO_JUSTFILE" --working-directory "$MEMO_OWNER_ROOT" _cargo check "$@"

test module="all" *args:
    @just --justfile "$MEMO_JUSTFILE" --working-directory "$MEMO_OWNER_ROOT" _cargo test "$@"

_cargo action module *args:
    #!/usr/bin/env bash
    set -euo pipefail
    action=$1; module=${2%.ctg}; shift 2
    repos=$(dirname "$MEMO_OWNER_ROOT")
    export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$MEMO_OWNER_ROOT/target}
    export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
    export CARGO_PROFILE_DEV_INCREMENTAL=false CARGO_PROFILE_TEST_INCREMENTAL=false
    if [[ "$module" == all ]]; then
        if (( $# )); then echo 'Select an owner before passing extra arguments' >&2; exit 2; fi
        for owner in runtime workspace memory policy ui live auth prd workspace-ui; do
            just --justfile "$MEMO_JUSTFILE" --working-directory "$MEMO_OWNER_ROOT" _cargo "$action" "$owner"
        done
        if [[ "$action" == check ]]; then
            "$MEMO_OWNER_ROOT/.cartridge/tools/memo-run" "$repos/.cartridge/memos/routine/check-cartridge-layout.md" check
        fi
        exit
    fi
    case "$module" in
        workspace-ui)
            cd "$repos/workspace.ctg"
            if [[ "$action" == build ]]; then exec bun install --frozen-lockfile "$@"; fi
            exec bun run "$action" "$@";;
        ui|live|auth|prd)
            cd "$repos/$module.ctg"
            if [[ "$action" == build ]]; then exec bun install --frozen-lockfile "$@"; fi
            exec bun run "$action" "$@";;
        runtime|cartridge) manifest="$MEMO_OWNER_ROOT/Cargo.toml"; scope=(-p cartridge);;
        memory) manifest="$repos/memory.ctg/Cargo.toml"; scope=(--workspace);;
        memory/*) manifest="$repos/memory.ctg/Cargo.toml"; scope=(-p "${module#memory/}");;
        workspace) manifest="$MEMO_OWNER_ROOT/.cartridge/workspace/Cargo.toml"; scope=(--workspace);;
        policy)
            exec "$MEMO_OWNER_ROOT/.cartridge/tools/memo-run" "$repos/policy.ctg/.cartridge/memos/routine/policy-tests.md" test "$@";;
        agent|docs|fs|gitfs|harness|landscape|mcp|memo|memory-tool|proxy|pty|router|sessions|tools)
            manifest="$MEMO_OWNER_ROOT/.cartridge/workspace/Cargo.toml"
            package=$module; [[ "$module" != memo ]] || package=memo_cartridge
            scope=(-p "$package");;
        *) echo "Unknown owner: $module" >&2; exit 2;;
    esac
    if [[ "$action" == check ]]; then
        if [[ "${scope[0]}" == -p ]]; then format=("${scope[@]}"); else format=(--all); fi
        cargo fmt --manifest-path "$manifest" "${format[@]}" -- --check
        exec cargo clippy --manifest-path "$manifest" "${scope[@]}" --all-targets "$@" -- -D warnings
    fi
    cargo "$action" --manifest-path "$manifest" "${scope[@]}" --all-targets "$@"
    if [[ "$action" == test && "$module" == runtime ]]; then
        bun test "$MEMO_OWNER_ROOT/.cartridge/tests/integration/memo-run.test.ts"
    fi
    if [[ "$action" == test && ( "$module" == workspace || "$module" == tools ) ]]; then
        bun test "$repos/tools.ctg/.cartridge/tests/integration/lane.test.ts"
    fi
    if [[ "$action" == test && "$module" == memory && $# == 0 ]]; then
        cargo test --manifest-path "$manifest" --workspace --doc
    fi

links:
    #!/usr/bin/env bash
    set -euo pipefail
    export CARTRIDGE_REPOSITORIES=$(dirname "$MEMO_OWNER_ROOT")
    export CARTRIDGE_TARGET=${CARGO_TARGET_DIR:-$MEMO_OWNER_ROOT/target}
    bun -e '
      const fs=require("node:fs"), path=require("node:path");
      for (const name of fs.readdirSync(process.env.CARTRIDGE_REPOSITORIES).filter(n=>n.endsWith(".ctg"))) {
        const owner=path.join(process.env.CARTRIDGE_REPOSITORIES,name), file=path.join(owner,"cartridge.json");
        if(!fs.existsSync(file)) continue;
        const m=JSON.parse(fs.readFileSync(file,"utf8")), binary=m.binary||m.name;
        if(!binary || binary.includes("/")) continue;
        const target=path.join(process.env.CARTRIDGE_TARGET,"debug",binary);
        if(!fs.existsSync(target)) continue;
        const output=path.join(owner,".cartridge/bin",binary);
        fs.mkdirSync(path.dirname(output),{recursive:true});
        if(fs.existsSync(output)||fs.lstatSync(output,{throwIfNoEntry:false})) {
          if(!fs.lstatSync(output).isSymbolicLink()) throw Error("Refusing to replace "+output);
          fs.unlinkSync(output);
        }
        fs.symlinkSync(path.relative(path.dirname(output),target),output);
      }'
```

## Proof and recovery

Run one owner first, then the full composed checks. Binary links point only at
existing outputs; a real local executable is never overwritten. Build failures
preserve sources, records, credentials and stores. Generated binaries and logs
are not records and must not be committed.
