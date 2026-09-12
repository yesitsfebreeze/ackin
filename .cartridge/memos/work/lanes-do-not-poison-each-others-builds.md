---
kind: work
description: "A build in one lane worktree cannot fail a sibling lane's build"
status: done
level: 10
priority: P1
estimate: 4h
---

# lanes-do-not-poison-each-others-builds

## Outcome

Two lanes building at the same time do not break each other. Today
`.cargo/config.toml` points every worktree at one shared `target/`, so a sibling
lane's build hands a stale `zirkle` rlib to a dependent crate and the failure
surfaces as a compile error in untouched code:

```
error[E0599]: no method named `inventory` found for reference `&zirkle::sdk::Host`
```

seen from `cargo nextest run -p memo_cartridge` and from
`cargo build --workspace --bins` against `builtin/memo/src/service.rs`. `touch
core/sdk.rs` clears it. Three independent agents hit it in one work pass and two
spent a detour each deciding whether the error was theirs; one of the same passes
filled the disk, because every lane also carries a full duplicate build.

The fix is whatever makes the shared target safe or unshared — a per-worktree
`CARGO_TARGET_DIR`, a shared build directory cargo actually locks against, or
`just lane` provisioning one. Whatever is chosen, the trap must stop presenting
itself as a type error in code nobody edited, and disk cost is part of the
choice: a dozen full target directories is its own failure.

## Check

- [x] Two cold lane worktrees both exit 0 from `cargo build --workspace --bins`
      run concurrently, with no `touch` between them (observed 2026-09-12:
      191 crates each, through the provisioned configs).
- [x] With lane A's `core/sdk.rs` renaming `cartridges`, running
      `cargo build -p zirkle --lib` in A and then, after
      `touch builtin/memo/src/record.rs`, `cargo nextest run -p memo_cartridge`
      in B passes B's 31 tests — the same sequence against one shared target
      dir fails with E0599 in `builtin/memo/src/service.rs`.
- [x] `just lane <name>` writes `<lane>/.cargo/config.toml` naming
      `<lane>/.cargo-target`, rerunning it leaves an existing config alone,
      and `lane-rm` removes the lane while any other file in `.cargo/`
      blocks removal.
- [x] `just check` and `just test` pass on the lane.
- [x] The disk cost of the chosen scheme is stated here, measured on this
      tree (2026-09-12, remeasured): 1.5-1.7 GB per lane for a full dev
      build, 328 MB for the `-p memo_cartridge` build scope (358 MB with
      nextest's test binaries), deleted with the lane by `lane-rm`.

## Approach

`just lane` provisions each worktree with `<lane>/.cargo/config.toml` whose
`build.target-dir` is that worktree's own `<lane>/.cargo-target`. A shared
target cannot be made safe, only unshared: cargo decides freshness between two
checkouts by mtime alone and writes both checkouts' rlibs under one filename,
so the shared directory hands a sibling's artifact over in both directions.
Rerunning `just lane <name>` backfills the config on an existing worktree and
never rewrites one that is already there (a person's override).

Pass one is implemented, tested, and committed on
`work/lanes-do-not-poison-each-others-builds` (faa8a6d):

1. `builtin/tools/service.rs` — `provision()` writes the config after
   `git worktree add` and on existing lanes; `is_build_artefact` takes
   `.cargo-target` and `.cargo/config.toml`; `lane-rm` expands the collapsed
   `.cargo/` listing, so the provisioned config is stepped over while any file
   a person puts beside it still blocks removal.
2. `builtin/tools/tests/test_lane.py` — provisioning, idempotence, and
   `lane-rm` removal; all 9 lane tests pass against the rebuilt binary.
3. `.gitignore` — `/.cargo-target/`.
4. `justfile` — the `target :=` comment names the provisioning.

What is left, in order, from the lane:

    just check
    just test

Verification the probe already ran (2026-09-12, on this tree):

    # check 1 — two cold worktrees build the whole workspace concurrently:
    cargo build --workspace --bins   # in lane A and lane B, at the same time
    # both exited 0, 191 crates each
    # check 2 — the poison sequence under isolation:
    sed -i '' 's/pub async fn cartridges(/pub async fn renamed(/' core/sdk.rs  # A
    touch core/sdk.rs && cargo build -p zirkle --lib                           # A
    touch builtin/memo/src/record.rs                                           # B
    cargo nextest run -p memo_cartridge                                        # B

The same sequence against ONE shared target dir fails exactly as the Outcome
records: `error[E0599]: no method named 'cartridges' ... &zirkle::sdk::Host` in
`builtin/memo/src/service.rs`, from B's own harmless edit, and
`touch core/sdk.rs` clears it. The poison also appears passively: cargo
declared a divergent checkout's zirkle "Finished in 0.03s" and handed it the
sibling's rlib without building, and a forced rebuild overwrote
`libzirkle-0ab87ce99217f078.rlib` under the same filename. Both observations
were made with this machine's `rustc-wrapper = "kache"` active, so the trap
sits in cargo's fingerprint layer, not the compiler wrapper.

Disk cost, measured on this tree (2026-09-12): one lane's full-workspace dev
target is 1.4 GB (a second checkout measured 1.7 GB); the `-p memo_cartridge`
scope with dependencies is ~220 MB. The cost is bounded by live lanes —
`lane-rm` deletes the target with the worktree — against a trunk target that
stood at 84 GB before the same pass cleaned it. A dozen live lanes is about
17 GB.

## Result

Landed on `work/lanes-do-not-poison-each-others-builds`, implementation
commit faa8a6d: `just lane` provisions every worktree with
`<lane>/.cargo/config.toml` naming that worktree's own
`<lane>/.cargo-target`, backfills the config on existing lanes, never
rewrites one a person put there, and `lane-rm` steps over the provisioned
pair while any other file in `.cargo/` still blocks removal
(builtin/tools/service.rs, builtin/tools/tests/test_lane.py, .gitignore,
justfile comment). The trunk's own `.cargo/config.toml` still pins its
trunk-wide target; that file belongs to the trunk owner and was not
touched.

Verified 2026-09-12 on this tree: two cold provisioned lanes ran
`cargo build --workspace --bins` concurrently, both exit 0, 191 crates
each; the poison sequence (divergent `cartridges` rename built as
`-p zirkle --lib` in lane A, then `touch builtin/memo/src/record.rs` and
`cargo nextest run -p memo_cartridge` in a cold lane B) passed 31/31 with
no E0599, against the shared-target failure recorded in the Approach.
`just check` and `just test` both exit 0 on the lane (307/307 workspace
tests, 1337 memory tests, doc tests, python suites, 9 lane tests,
compaction gate; one earlier `just test` run under three concurrent
sibling-lane suites failed two tests to a SIGKILLed fixture and a 30s
apply timeout — both load artifacts that passed in isolation and in the
full re-run). Disk cost, remeasured: 1.5-1.7 GB per lane for a full dev
build, 328 MB for the `-p memo_cartridge` build scope (358 MB with
nextest's test binaries), reclaimed by `lane-rm` with the lane.
