# the-resolver — pass three (implementer, mason, retry)

Verdict: DONE

The retry's wall is cleared: the 8 paths the 2026-09-12 19:53 failure named
are all inside the widened spec footprints now (spec01: `src/resolver.rs`,
`src/lib.rs`, `src/tests/resolver.rs`, `src/tests/mod.rs`; spec02:
`src/main.rs`, `src/cartridge.rs`; spec03: `src/main.rs`, `src/loader.rs`,
`src/sdk.rs`, `src/tests/fixtures/chain_fixture.rs`, `Cargo.toml`) — nothing
was dropped, and `src/loader.rs` / `src/sdk.rs` did not move anyway.

## The lane

- The probe's uncommitted pass-one/pass-two tree is committed:
  `62d679f the-resolver: the walk is the launch path` (8 files, +1067/−2).
- `git rebase session/s25699` — clean, no conflicts; the lane was already
  based on `session/s25699` (`790b548`), so the branch is exactly one commit
  ahead of it.
- Note for the collect: `session/s25699` (`790b548`) is two commits behind the
  lane's `main` (`213d2ab the-ledger — the skeptic's corrections`). The
  instruction named `session/s25699` and was followed; if the board wants the
  resolver on the corrected ledger, a later rebase of `62d679f` onto `main`
  should be the collect's move.

## Box status (all re-run on the committed tree)

- `specs/spec01.md` — 4/4 `[x]`, quotes match:
  `cargo test --lib tests::resolver::` → **12 passed** (walk: bottom-up chain,
  nested provider before an outer lookalike, `Refusal::Cycle { a.key, at: "a" }`,
  `Ambiguous` naming every offer, `Unbound` naming key and asker, diamond
  carrying `shared` once).
- `specs/spec02.md` — 5/5 `[x]` (box 2 carries its supersession note; the
  refusal moved to a declared `binary` that does not exist). Probe legs:
  `db ppid=1`, `store ppid=db`, `tool ppid=store`; removed-node re-entry
  refuses ("is no longer installed: nothing launches"); cycle →
  "refused: `a.key` resolves into `a` again: cycle"; captured-stdout ask
  returned with the whole tree up.
- `specs/spec03.md` — 4/4 `[x]`. Probe hosted legs: `db pid ppid=1 (hosted)`,
  `store ppid=db (hosted)`, `tool ppid=store (hosted)`; "dependents still
  alive after the hosted far end died: 0"; missing-binary refusal naming the
  node and the program; the full `cargo test --lib` → **87 passed**.

## Verify and Proof (run 2026-09-12, post-commit, post-rebase)

```sh
cargo test --lib tests::resolver::  # 12 passed; 0 failed
cargo test --lib                    # 87 passed; 0 failed
cargo clippy --all-targets          # clean (warnings = "deny")
sh .pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh  # exit=0
```

## What moved this pass

Nothing in `src/`: the standing tree was complete against all three specs
(including the hosted route); this pass committed it, rebased it, and
re-verified every box. The lane was left one commit ahead of
`session/s25699`, tree clean.

## Health floor

"none under the floor" — nothing to lift. One pre-existing defect outside
scope, reported not fixed: `cargo fmt --check` fails repo-wide (276 diff
blocks across every source file) because the codebase is hard-tab-indented
and there is no `rustfmt.toml` setting `hard_tabs = true`; rustfmt's default
is 4 spaces. Present on committed files untouched by this PRD (`ledger.rs`,
`loader.rs`, `turn.rs`, …), so it is not this PRD's making; a one-line
`rustfmt.toml` is the fix whenever someone takes it.

The two earlier passes' reports are preserved below, unedited.

---

# the-resolver — pass two (implementer, mason)

Verdict: DONE

The specs stand in the lane at `/Users/feb/dev/cartridge/.pearde/.lanes/the-resolver`,
uncommitted on top of pass one (lane commit `790b548`). The repo's gate is
`cargo clippy --all-targets` under `warnings = "deny"` — clean — and
`cargo test --lib` is green: **87 passed** (75 baseline unchanged, +3 net new;
one pass-one test was replaced by hosting, three added). The probe
(`.pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh`) is green
end-to-end and now also carries a hosted leg.

## Box status

- `specs/spec01.md` — 4/4 ticked. The walk tests
  (`cargo test --lib tests::resolver::` → 12 passed): bottom-up chain,
  nested provider before an outer lookalike, `Cycle` naming key and node,
  `Ambiguous` naming every offer, `Unbound` naming key and asker, diamond
  carrying `shared` once.
- `specs/spec02.md` — 5/5 ticked, one with a supersession note: box 2 (a
  no-binary cartridge refuses the ask) was run and passed at this spec's
  state, and spec03 replaces the refusal with hosting. The refusal now
  belongs to a *declared* `binary` that does not exist.
- `specs/spec03.md` — 4/4 ticked, built this pass:
  `a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency`
  (tree, cascade, uninstall-absence, all through hosted nodes),
  `a_chain_node_whose_binary_does_not_exist_refuses_the_ask`,
  `a_reentry_into_an_uninstalled_node_launches_nothing` (closes spec02 box 3,
  which nothing tested before), and the probe's hosted leg.

## What moved

- `src/main.rs` — `node_program` became `Route` (`Program | Hosted`) +
  `node_route`: a document with a `binary` resolves it; one without is
  **hosted** — the binary itself is the program — and the ask resolves the
  Lua entry (`Cartridge::read`, data not evaluation) so a missing entry file
  still refuses at the ask. A hidden `zirkle node` mode is the hosted
  program: it loads the cartridge's Lua component in-host
  (`Host::component`), registers its provides the way an in-host fiber does
  (`ctx.provide` in the apply), then re-enters the binary for the next link
  exactly as the program fixture does (held child with `kill_on_drop`,
  stdin watched for the EOF cascade, `ZIRKLE_BOTTOM` on the far end only).
  `enter` execs into the node mode for hosted nodes, so there is still no
  shim process; the dependency remains the direct parent.
- `src/tests/resolver.rs` — the no-binary refusal test replaced by the
  hosted twins; a missing-program refusal test; a re-entry-into-uninstalled
  refusal test (new); the fixture-route ask now runs under `.output()`, so
  its returning at all is the proof of spec02 box 5's captured-stdout claim.
- `src/tests/fixtures/chain_fixture.rs` — one-line clippy fix
  (`format!("{}", …)` → `to_string()`); **`cargo clippy --all-targets` was
  not clean at the start of this pass** (the example failed the
  `useless_format` lint under the deny-warnings gate), despite the pass-one
  report's claim. Fixed; clean now.
- probe — a hosted section: a no-binary chain up from the bottom, `ps`
  showing the hosted tree (`store ppid=db`, `tool ppid=store`), teardown
  through the hosted chain (0 survivors after the hosted far end is killed),
  and reaping in the trap.

## Findings

1. **The needs of a hosted node are the chain's business, not injections.**
   The document's `provide` becomes the provision, and the apply registers
   the values (tested in-process by
   `a_hosted_node_runs_its_lua_component_and_provides_its_keys`). But the
   component's *needs* are cleared from the fiber's injections: carried as
   inject keys they would hold the apply waiting for providers that live in
   other processes and can never arrive in this one, so a real chain's tool
   would come up inert. The chain is the resolution the resolver already
   made — the dependency launched the node. A call to a need refuses at the
   call site (`Inactive`/`Undeclared`) instead of stalling the node.
2. **The cross-node call wire is the open boundary.** Nothing today carries
   a call from one chain node's Lua to another chain node's provide. That is
   the wire PRD's territory (the spec's open list already defers detached
   diagnostics and ready signalling there); until it exists, a hosted chain
   is a correct tree whose nodes provide into their own processes only.
3. **A Lua entry returning a `zirkle.process` descriptor is mis-hosted.**
   The descriptor's component spawns its process against a `Link` nothing
   serves in a detached node. It is refused nowhere and mis-handles at run
   time. Narrow, but real; a hosted node could refuse such an entry by name
   at the ask if the ask ever evaluates the entry — same "ready signalling"
   question as the rest.
4. **A hosted node whose apply fails stays up** as the tree link (a broken
   provider that keeps its dependents alive); one whose component fails to
   *load* exits before re-entering, so its dependents never spawn. Both are
   recorded here rather than resolved — the right behaviour belongs with
   ready signalling.
5. Footprint note: spec03's footprint named `loader.rs` and `sdk.rs`;
   neither moved — hosting lives in `main.rs`, the loader's and SDK's
   contracts untouched.

The pass-one report (analyst) is preserved below, unedited.

---

# the-resolver — pass one (analyst, build-then-spec)

Verdict: SPECCED

The mechanism is built and demonstrated, not spec'd from reading. The lane at
`/Users/feb/dev/cartridge/.pearde/.lanes/the-resolver` holds the work
uncommitted (pass one, on top of lane commit `790b548`): 84 tests green (75
baseline + 9 new), `cargo clippy --all-targets` clean under
`warnings = "deny"`, and an end-to-end probe green over real processes
(`/Users/feb/dev/cartridge/.pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh`).

What stands:

- `src/resolver.rs` — `chain(&ledger, from, key)`: the walk to the far end,
  bottom-up, over the ledger's outward lookup (nested subtree answers before
  an outer lookalike); `Refusal::Unbound | Ambiguous | Cycle` with naming
  diagnostics; the cycle stack inherited from `deps()`'s duty; a diamond
  carries its shared provider once.
- `src/main.rs` — `zirkle up <key>` resolves once, pre-checks every node's
  program, spawns the far end detached (stdio null) and exits;
  `zirkle enter <node> --rest <tail>` re-reads the fresh ledger and **execs**
  into the document's `binary` — no shim process, the dependency is the direct
  parent. Env protocol: `ZIRKLE_NODE`, `ZIRKLE_CHAIN`, `ZIRKLE_ZIRKLE`,
  `ZIRKLE_ROOT`, `ZIRKLE_BOTTOM` (far end only; `enter` strips it).
- `src/cartridge.rs` — the document's `binary` field's first real consumer
  (`executable` went `pub` for it); a node with no program is refused by the
  ask, before anything spawns.
- `src/tests/fixtures/chain_fixture.rs` (example target) — the SDK stand-in:
  re-enters the binary for its dependent, holds the child with
  `kill_on_drop`, watches its stdin for the EOF cascade; writes a pid marker
  the tests observe.
- The probe demonstrates, over real processes: one process per node, each
  dependent a child of its dependency (`ps` ppid asserted); killing the far
  end takes every dependent with it (0 survivors); uninstalling is the
  absence of a launch (fresh-ledger refusal); a cycle is refused on the
  launch path.

## Specs

- `specs/spec01.md` — the walk is the launch plan; refusals (complexity 5;
  stands)
- `specs/spec02.md` — the ask spawns the far end and exits; every link
  re-enters and execs (complexity 8; stands)
- `specs/spec03.md` — a Lua-entry cartridge is hosted: the binary is the
  program a chain node runs (complexity 12; **not built** — the one unit the
  probe could only stand in for)

Sum 25 (limit 40), count 3 (limit 6).

## Scores

complexity: 65
blast-radius: mid
workflow: probe-then-spec

## Reasoning

- complexity 65 — the walk and the refusals reuse the ledger's settled
  pieces, but the PRD carries the system's core property (the process tree is
  the dependency tree, nothing maintains it) through process semantics that
  are new and easy to get subtly wrong: exec hand-off, the stdio-EOF
  teardown cascade, the detached far end, and the env protocol.
- blast-radius mid — the launch path is new front-door surface that every
  PRD which starts a tool will consume, but nothing standing changes
  behaviour: the ledger's contract, the in-host runtime, and the existing
  verbs are untouched (75 baseline tests green unchanged).
- workflow probe-then-spec — an open PRD specced from a build; all five steps
  ran (contract, baseline, build, harnesses, specs).

## Union of footprints

`.pearde/.lanes/the-resolver/src/resolver.rs`, `src/lib.rs`,
`src/tests/resolver.rs`, `src/tests/mod.rs`, `src/main.rs`,
`src/cartridge.rs`, `src/loader.rs`, `src/sdk.rs`

## Findings

1. **The walk is the launch, one invocation per link.** `up` resolves the
   whole chain (a refusal is a refusal of the launch: nothing spawns), spawns
   the far end detached and exits; each node re-enters via `enter`, which
   re-reads the fresh ledger and execs into the program. Nothing sits above
   the tree; teardown is the dependency's exit, ordered for free; uninstall
   is the absence of a launch. All demonstrated by the probe.

2. **The teardown cascade is a pipe, not a protocol.** The exec'd node
   inherits its dependency's stdin pipe; when the dependency dies the pipe
   closes, EOF makes the node exit, and its held child (`kill_on_drop`)
   takes the next link along. Three traps found on the way, recorded in
   [[260912-5366]]: a detached node must not inherit the asker's stdio (a
   captured caller hangs forever); `kill_on_drop` kills on the drop of a
   `Child` handle, and a bare `spawn()?` temporary drops at statement end;
   setting `.stdin(piped())` on the `enter` invocation makes a second pipe
   whose write end dies at exec — inherit instead. `ZIRKLE_BOTTOM` marks the
   far end (it watches nothing and dies only when killed) and `enter` must
   strip it so it does not leak into dependents.

3. **`binary` found its consumer.** The field was parsed and validated since
   its introduction and read by nothing; the resolver is its first reader.
   A chain node whose document declares no binary is refused at the ask,
   naming the node, before anything spawns. But every cartridge in the tree
   today is Lua-entry with no `binary`, so spec03 hosts them: the binary
   itself is the program a chain node runs. Open and out of scope for now:
   where a detached node's diagnostics go (stdio null — telemetry/wire
   territory), ready signalling, already-running detection.

4. **The flat-search parallel is settled on the launch side, untouched in
   host.** The PRD names `src/loader.rs`'s inject search as the thing this
   changes; the build made the *launch* lookup the ledger's outward walk, and
   the in-host fiber world already scopes by realm ("already provided in this
   realm"), so the loader's flat list stands for readiness and display only.
   If a later pass wants in-host inject binding subtree-scoped, that is its
   own change and is not needed by this mechanism.

5. **Refusals on the launch path.** Unbound (legal to list, not legal to
   launch), Ambiguous (one scope, two offers — names every offer), Cycle
   (closes on the walk's own stack). A missing program at any link refuses
   before anything spawns, because `up` pre-checks the whole chain first.
