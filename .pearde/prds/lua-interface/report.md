Verdict: SPECCED

# lua-interface — analyst pass (quill, 2026-09-12)

The build went through: a new cartridge can now be written, wired and run
without touching Rust, proven end-to-end over real processes
(`.pearde/prds/lua-interface/probe/write-wire-run.sh` — six files written, the
ledger reading the tree, one ask up, two calls answered across the chain, the
dependent gone with its dependency). Lane
`/Users/feb/dev/cartridge/.pearde/.lanes/lua-interface`: 89 passed, 0 failed,
clippy clean. The lane's git history carries the port as commit `6fe121c`
(base); this PRD's own delta sits on top uncommitted, with a copy under
`prds/lua-interface/probe/src/`.

## What stands

- **The chain link** (the wire across chain nodes, named open by the
  resolver's pass one): each node serves its provided keys on a socket derived
  from root + ledger path (`socket::node_path`), connects to its dependency's
  socket (`ZIRKLE_DEP`, set when the dependency re-enters), binds every
  document need to a remote over it (`Host::bind_dependency`), and
  `ctx:get` of a need resolves to that remote — in the node's fiber and in
  every fiber nested under it. The node serves its own keys on its per-node
  socket; a key bound from a dependency is reached through the dependency's
  socket, not fronted by the node's own — a bound need called on the node's
  socket refuses `` `<key>` is not provided ``, and answers only through
  `ctx:get` in Lua (`Host::call` is a bare store peek). The stdin pipe stays
  the kill channel; the sockets carry the calls; neither depends on the
  other.
- **The document carries its own config** (`loader::Cartridge.config` to
  `Declared` to a `load_component` fallback): no profile `config.lua` in the
  ledger shape, so the author's configuration travels in `cartridge.json`;
  a caller that names its own lays it over.
- The port: the resolver's pass-one chain (`up`/`enter`/`node`,
  `resolver.rs`, its tests and fixture) as this lane's base — 87 green before
  this PRD's first edit.

## Specs

- `specs/spec01.md` — the chain link: one socket per node, needs bound to
  remotes over it, `ctx:get` resolving through it. Complexity 16; stands.
- `specs/spec02.md` — the document carries the cartridge's own config.
  Complexity 5; stands.

Spec sum 21, count 2 — under both board limits.

- complexity: 30 — the chain link is a new cross-process channel with
  lifecycle coupling (bind order, retry, gone-failure), but it is
  assembled from four standing mechanisms — the settled socket transport, the
  wire's reply envelope, the resolver's chain walk, the ledger's bindings —
  and the document config is one field.
- blast-radius: mid — the node mode and the Lua `get` surface gain behavior
  only where a chain link or a document config exists; the daemon, the old
  profile shape and all 87 ported tests run unchanged, while the surface every
  Lua cartridge is written against gains a new path in.
- workflow: wire-the-chain

Footprint union: `.pearde/.lanes/lua-interface/src/main.rs`,
`src/lua.rs`, `src/context.rs`, `src/socket.rs`, `src/cartridge.rs`,
`src/loader.rs`, `src/tests/node.rs`, `src/tests/mod.rs`,
`src/tests/resolver.rs` (helper visibility only), `Cargo.toml` (from the
port).

## Findings

1. **The record tooling is missing on this machine** — as the-wire's analyst
   already recorded: no `resources/knowledge.py`, `workflows.py`,
   `grammar.py`, `questions.py`, no `@references/`. The query could not run
   and could not enqueue a gap; one gap is already pending
   (`.pearde/wiki/pending/260912-6709.md`). The workflow library survives as
   plain md under `.pearde/workflows/`, and is what I followed.
2. **The base is a sibling's uncommitted pass one.** This PRD's surface sits
   on the resolver's chain (`up`/`enter`/`node`), which exists only uncommitted
   in the-resolver's lane. I ported it into this lane as base commit
   `6fe121c` and built on it. When the-resolver lands, this lane's port should
   be dropped in favour of the landed commits — the port is a stand-in base,
   not a second implementation to maintain.
3. **The in-host launcher still assumes the profile.** `zirkle run`,
   `verify` and `reconcile` load nothing without a profile `init.lua` — the
   ledger-derived entries are all disabled by the-ledger's settled
   available-not-started answer, and nothing re-enables them. The chain is now
   the launcher of the new shape, so the PRD's promise holds, but the in-host
   foreground runner cannot launch a ledger cartridge. Whether that runner
   learns the ledger walk or is cut (the standing direction: remove what
   nothing uses) is its own call, not specced here.
4. **A parent cannot `get` a key its own composed child provides.** `get`
   walks the declaration chain, and a parent that injected its child's key
   would stall waiting for a provider only its own apply can compose. Today
   the composed child's key answers through `ctx:peek` (bare store lookup) and
   through the node's socket; the surface asymmetry is real and worth knowing
   before anyone widens `get`.
5. **Events do not cross the chain link.** `ctx:emit`/`on` stay in-process on
   each node; a node's outbox reaches only its own socket clients. The
   telemetry PRD's channels are where cross-node events belong — left open,
   deliberately.
6. **A second ask spawns duplicate nodes.** No already-running detection on
   the hosted route (the resolver named it open); the second node's socket
   serve fails with "a daemon already serves this directory" but the process
   stays up. The per-node socket makes detection cheap now; not built here.
7. **Board copy stale.** `.pearde/wiki/board/lua-interface.md` still reads
   `state: open · complexity 0 · blast low`; the PRD frontmatter says
   `analyzing`. Not mine to edit.

## Route

## Use when

- A PRD's surface sits on a launch path that exists only as a sibling lane's
  uncommitted pass one, and the build has to bring that base across before it
  can add its own mechanism on top.
- Not when pass one already ran in this lane and the PRD carries `## Answers`
  — that is `probe-then-spec`, which continues past a closed fork instead of
  porting a base first.

## Steps

| # | atomic | why | on failure |
|---|--------|-----|------------|
| 1 | `read-the-contract` | the PRD's promise read against the resolver's pass one located the one open seam this PRD owns: a call to a need had no channel and refused at the call site | `stop` |
| 2 | `query-the-record-first` | attempted as the contract demands; the record tool is missing on this machine, already a recorded finding | `→ 1` |
| 3 | `port-the-pass-one-base` | the chain exists only uncommitted in a sibling lane; ported, committed, 87 green — the base this surface's diff is honest against | `→ 1` |
| 4 | `attempt-the-build` | the socket derivation, the binding, the env addition and the config field are compiler-read work; 89 green after | `→ 3` |
| 5 | `prove-the-chain` | a shell probe over real processes plus a regression test: the call across the chain, the composed cartridge through the same socket, the cascade | `→ 3` |
| 6 | `write-the-specs` | the standing work split into two re-verifiable units | `→ 3` |

### atomic port-the-pass-one-base

## Do

1. Copy the sibling lane's uncommitted pass-one files into this lane
   (`resolver.rs`, the tests, the fixture, and the touched
   `main.rs`/`cartridge.rs`/`lib.rs`/`Cargo.toml`/`tests/mod.rs`), taken from
   the sibling's working tree, not from any commit.
2. Register the new module in `lib.rs`, commit the port as the lane's base,
   and run the full suite before touching anything this PRD owns.

## Done when

- The suite is green at the ported count (87 here) and the lane's git log
  names the port as its own commit, so the PRD's delta reads as a diff.

## Fails when

### atomic prove-the-chain

## Do

1. Write the probe as a shell script under `prds/<prd>/probe/` that builds the
   lane, writes the fixture tree into a `mktemp -d` root (never under
   `.pearde/prds/`), runs the ask, derives the socket from the ask's own
   output, and calls it; reap the detached nodes in the cleanup trap.
2. Write the same proof as a regression test in the lane, spawning the real
   binary (not a fixture stand-in) with `ZIRKLE_NODES` for observation, and
   asserting the cross-chain call, the composed cartridge's answer, the
   refusal of an unprovided key, and the cascade after the dependency's death.

## Done when

- The probe and the test both end on the dependency's death taking its
  dependent along, not merely on a call that stopped answering.

## Fails when

## Scores

complexity: 30
blast-radius: mid
workflow: wire-the-chain

Verdict: DONE

# lua-interface — implementer pass (wren, 2026-09-12)

Both specs stand and every acceptance box is closed with quoted output:
`cargo test --lib` 97 passed / 0 failed (89 at spec time; the-wire's landed
`tests::wire` adds 8), clippy clean on the whole footprint, and the probe
green over real processes including the unprovided-key refusal
(`{"error": "`nowhere` is not provided", "reply": 1}`) and the cascade.

## Workflow wire-the-chain

| # | atomic | why | on failure |
|---|--------|-----|------------|
| 1 | `read-the-contract` | the PRD promise re-read against the tree: the specs name two units, both written from the probe copy — and the census was re-checked, which caught that the lane tree no longer held the pass-one code | `stop` |
| 2 | `query-the-record-first` | still impossible: no `resources/knowledge.py` on this machine (recorded finding, gap already pending) | `→ 1` |
| 3 | `port-the-pass-one-base` | superseded: the-resolver, the-ledger and the-wire landed since the spec was written (lane HEAD `ab7930e`), so the stand-in port is gone and the base is the landed chain — the suite is green at 97 against it | `→ 1` |
| 4 | `attempt-the-build` | delta re-applied on the newer base, `cargo build -p zirkle` compiles the binary and lib after a forced `touch src/main.rs src/lib.rs` recompile (`Compiling zirkle` confirmed) | `→ 3` |
| 5 | `prove-the-chain` | the probe extended with the unprovided-key call and run green; the lane regression test green (`tests::node::` 2 passed / 0 failed) | `→ 3` |
| 6 | `write-the-specs` | specs existed unchanged — no new units written; both specs' boxes ticked at close with the command and its output | `→ 3` |

### Edits

- `cargo test --lib tests::node::` first failed with two panics
  (`unknown field config` / "`tool.hello` asked for at `` binds to nothing
  in scope"): the lane working tree had been reset to the landed tip at
  ~20:18, wiping the uncommitted pass-one delta from `loader.rs`,
  `context.rs`, `lua.rs`, `main.rs`, `cartridge.rs` (only `socket.rs` and the
  tests survived). Re-applied the delta from the probe copies — see Findings.
- The probe script gained one call (the unprovided key) so spec01's refusal
  box could be closed on real output, not on an assertion of `reply: 1`
  alone.

## Findings

1. **The uncommitted pass-one delta was wiped, then restored.** At ~20:18
   today the lane working tree was reset to the landed tip (the-wire's
   collect commit `ab7930e` at 20:17 touched `cartridge.rs`); `git status`
   went clean on `loader.rs`, `context.rs`, `lua.rs`, `main.rs` and
   `cartridge.rs`, which is why both node tests failed. The probe copies
   under `prds/lua-interface/probe/src/` preserved the delta and it is now
   back in the lane. **The delta is uncommitted again** — commit the lane
   promptly, or a second reset wipes it a second time. A misapplied first
   attempt (patching from the stale probe base regressed landed work) sits
   in `stash@{0}`; drop it.
2. **The probe copies are stale relative to the landed base.** They were cut
   at 20:03 against the ported base `6fe121c`, so `cartridge.rs` in the probe
   carries the pre-the-wire `manifest` signature and `loader.rs` carries the
   pre-the-ledger `passed_on`. The delta was re-applied by hand onto the
   landed versions, not by file copy; `Link::new/accept/shutdown` stay
   `pub` (the binary uses them) — the probe copy had them `pub`, the-wire had
   narrowed them to `pub(crate)` only because nothing in the binary called
   them yet.
3. **Clippy fails at HEAD, outside this PRD's footprint** — `src/sdk.rs:306`
   (`manual_contains`), `src/sdk.rs:348` (`map_identity`),
   `src/tests/wire.rs:163` (`unnecessary_to_owned`), all landed by the-wire's
   collect commit `ab7930e`. Not touched from here (another PRD's files);
   the lane's clippy gate is red until the-wire's implementer pass fixes
   them.
4. **`cargo fmt --check` is not this repo's gate** — the main repo itself
   shows 299 fmt diffs under default rustfmt (the tree indents with tabs and
   carries no `rustfmt.toml`). Not a defect introduced here; naming it so
   the gate question is settled once.
5. Finding 7 of the analyst pass stands: the board copy at
   `.pearde/wiki/board/lua-interface.md` still reads `state: open ·
   complexity 0 · blast low`. Not mine to edit.

## Corrections

The skeptic (`skeptic.md`) returned CHANGE on the record, not the code:
`Host::call` fronts nothing — it is a bare store `peek`, and a bound need
called on the node's own socket refuses `` `<key>` is not provided `` (the
chain-link bullet above now says so; the complexity bullet's "fronting" is
gone), and spec01 box 4's refusal message now has its assert
(`tests::node`'s chain test asserts the error names the unbound key).
Landed `89be348`.
