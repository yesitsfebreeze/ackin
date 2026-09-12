Verdict: DONE

# The wire — implementer pass (mason, 2026-09-12)

Continued the probe's uncommitted code in the lane
`/Users/feb/dev/cartridge/.pearde/.lanes/the-wire`. spec01 was standing work
re-verified; spec02 was implemented this pass. Full suite `cargo test`:
**81 passed, 0 failed** (was 76 — five new tests). Scoped verify
`cargo test --lib tests::wire::`: `test result: ok. 6 passed; 0 failed; 0
ignored; 0 measured; 75 filtered out; finished in 1.39s`. A fresh recompile
was confirmed (`Compiling zirkle` after touching `src/lib.rs`; not a cache
hit), zero warnings.

## Workflow nest-the-wire

| # | atomic | this pass |
|---|--------|-----------|
| 1 | read-the-contract | PRD + both spec files + pass-one report read; the census was re-checked against the tree: the lane's `sdk.rs` re-advertised every child `provide` (spec02's gap is real), and `cartridge.rs`/`tests/mod.rs`/fixtures stand as the report said |
| 2 | query-the-record-first | attempted first; the record tooling is still missing (`resources/` absent beside `/Users/feb/.local/bin/pearde`, no `knowledge.py` anywhere in the repo) — the gap is a report line, not a question |
| 3 | speak-the-host-half | standing in the lane from pass one; re-verified, not rewritten |
| 4 | attempt-the-build | `cargo test` clean over lib + binary; fresh recompile confirmed |
| 5 | prove-the-nest | five wire regression tests green, including the three-hop nest the pass-one test already held |
| 6 | write-the-specs | spec01/spec02 pre-existed and the contract did not change; no new units written — this pass closed the open boxes in place (the second-pass form step 6.0 names) |

### Edits

- spec01's fourth acceptance box (exits before `ready` / unreadable line) was
  a check that could not fail — no test existed for it. A test was added:
  `tests::wire::a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed`.
- The reload and bridge boxes cannot be driven through an intercepted
  component (no slot, so the daemon's reload machinery and profile grant
  cannot reach it); the tests drive the nest through a wrapper entry
  (`zirkle.process`) whose entry config carries `config.child` and
  `config.bridge`. Recorded here because neither spec names the shape.
- Shell is nushell on this machine: `2>&1` is `out+err>`, and `$env`
  interpolation broke inside string interpolation. Workflow text unaffected —
  both are shell-level, not atomic-level, failures.

## Spec status

- `specs/spec01.md` — 4/4 boxes `[x]`, each ticked with the command and
  output that closed it (see the boxes and the `## Proof` block in the file).
- `specs/spec02.md` — 3/3 boxes `[x]`, same. The verify command of both specs
  is the same scoped run quoted above.

## What moved (all inside the footprints)

- `src/sdk.rs` (spec01 + spec02):
  - `Host` gains `declared: Vec<String>` (its own provide declaration, handed
    in by `Cartridge::run`) and `children: Arc<Mutex<Vec<Child>>>` with
    `struct Child { link, reload }` captured from each child's `hello`.
  - **Declared fronting** (spec02.1): `relay` re-advertises a child's
    `provide` upward only when the declaration names it; an undeclared child
    key stays a private service of the sub-host. This makes the module header's
    existing claim true — pass one's code contradicted its own doc.
  - **Reload carried down** (spec02.2): `Cartridge::run`'s reload branch, when
    its own handler table is empty, forwards `{"reload": ...}` down to a child
    whose `hello` declared reload and answers above with the child's reply; a
    child that declared nothing still answers `null`. First declared child
    wins when there are several (spawn order).
  - **Bridge forwarded** (spec02.3): new `sdk::Host::bridge_call` pass-through
    requesting the same frame on the parent's link; `relay` answers a child's
    `{"bridge":"call",...}` with it, spawned under the child frame's turn. The
    profile check stays in the daemon, where `cartridge.rs`'s `handle` branch
    already lives.
  - Bookkeeping: the children registry is pruned on reader EOF and in the
    dispose finalizer.
  - **Standing defect fixed** (spec01 footprint): the dispose finalizer never
    set the reader's `stopping` flag, so disposing a sub-host reported its
    child's death upward as `nested: exited` although the code comment calls a
    disposal death "the disposal working". The finalizer now stores the flag
    before sending `dispose`. The existing regression test's dispose step
    already covered the frame flow; the misreport only ever showed in the
    outbox after dispose.
- `src/tests/wire.rs` (spec01 + spec02): five new tests — undeclared child key
  stays private; reload carried down (child's ok answer switches the
  generation, its refusal arrives as the daemon's report and the live
  generation stays); a child that declared no reload still answers null; a
  nested bridge call resolves against the daemon's grant (answers `3` through
  `p`'s `counter` with the grant, the daemon's own text
  `profile has not granted bridge access` without); a child that exits before
  ready or garbles its output fails the spawn with the exact texts.
- Untouched spec01 footprint paths that needed nothing:
  `src/cartridge.rs`, `src/tests/mod.rs`,
  `src/tests/fixtures/nested_fixture.rs`, `Cargo.toml`.

## Findings

1. **The record tooling is still missing.** Re-verified this pass: no
   `resources/` beside `/Users/feb/.local/bin/pearde`, no `knowledge.py` in
   the repo. The record query could not run and could not enqueue its gap.
2. **`Cartridge::run`'s `hello` hardcodes `reload: true`**, so every SDK
   cartridge declares reload. The "declared nothing still answers `null`"
   branch is reachable only through a hand-spoken wire peer (the tests use
   shell-script children). If an SDK cartridge ever needs to opt out of
   reload, that is a `Cartridge` builder question for a future PRD, not this
   spec's.
3. **Multiple reload-declaring children:** the parent forwards to the first
   one that declared. One frame, one reply — a nest with two declaring
   children answers from the older spawn. If a nest ever needs all children
   asked, that is a protocol choice, not a bug.
4. **A disposed child's death is now silent; a wanted child's death is still
   reported, not failed** (the fiber-failing shape of `cartridge.rs`'s reader
   remains the daemon's alone). Left as chosen behavior, as pass one already
   reported.

## Health floor

None under the floor; nothing to move inside the footprints. A split observed
but not done (defect outside scope): `sdk.rs` now holds the relay frame
dispatcher, the spawn lifecycle and the run loop in one file — the next
worker who touches it will read three concerns at once.
