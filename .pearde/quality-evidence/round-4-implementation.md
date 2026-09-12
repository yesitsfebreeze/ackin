# Integration correction iteration 4

The round-3 critique failed on incomplete bundle builds/missing executable acceptance and memory Docker recipes that omitted the sibling SDK.

## Tools correction

`tools.ctg` commit `56165b2614df5d00d0878613e53b78f95d59089e` changes only `service.rs` and its focused tests. Cargo metadata discovers and deduplicates all required workspace roots; parent/runtime/memory builds receive the same explicit target and profile from either parent or runtime context. The original standalone layout remains supported. In the composed layout the destination is parent `dist/cartridge`.

Packaging refuses absent explicitly named binaries and implicit Cargo cartridge executables, while preserving Lua-only cartridges. Disposable fixtures use real Cargo metadata to test parent/runtime context equivalence, original flat layout, and missing-binary refusal.

The implementer reported these final checks on the committed clean tree:

- `cargo test --locked --package tools`: 6 passed, 0 failed (0.32s).
- `cargo fmt --package tools -- --check`: exit 0.
- `cargo clippy --locked --package tools --all-targets -- -D warnings`: exit 0.

Builds used the shared parent target with incremental compilation and debug information disabled. Actual debug bundles passed from both runtime-launcher and parent direct-host contexts. The smoke gate verified 15 manifests and the runtime plus 13 cartridge executables, hash-matching their build outputs. After moving the bundle to a temporary directory outside the workspace, its ledger succeeded and a real SDK sessions.list call returned []; the bundle was then restored to parent dist/cartridge. Parent logs: validation/round-4-bundle-runtime.log, round-4-bundle-parent.log, and round-4-bundle-smoke.log.

## Memory Docker correction

`memory.ctg` commit `ddd4797` mounts memory at `/work/memory.ctg` and the sibling SDK read-only at `/work/cartridge.ctg`, running from the memory directory. Both recipes pass `just` dry runs (`validation/memory-docker-recipes.log` in the parent). The local Docker daemon is unavailable, so actual container execution remains untested. Remote CI and publishing remain unrun.

Runtime production code is unchanged from the round-2 measured candidate. Its timing evidence applies to the runtime only, not to every newly assembled cartridge or bundle build. Sandbox policy backends and launch authority remain open board work.
