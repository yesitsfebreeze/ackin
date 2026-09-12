# lua-interface — skeptic pass (2026-09-12)

Checked the landed work on main, not the lane. The source landed in
`11322b7` ("implementer work at collect": cartridge.rs, context.rs, loader.rs,
lua.rs, main.rs, socket.rs, tests/mod.rs, tests/node.rs, tests/resolver.rs);
`7367355` carries the PRD, specs, report and probe copies; `6ba5231` is the
record. `git status --short -- src/` is clean, and the lane commit `560f762`
that the prd's `commit:` line names exists and matches the landing
(`git diff 560f762 --stat` in the lane is empty).

## Runs

- `cargo test --quiet` → **97 passed; 0 failed** (matches the report's
  count: 89 at spec time + the-wire's 8 `tests::wire`).
- `cargo test --quiet --lib tests::node` → 2 passed; 0 failed; 95 filtered —
  exactly the two tests spec01/spec02 name.
- `cargo clippy --all-targets` fails only where the report already named it
  (`src/tests/wire.rs:163` `unnecessary_to_owned`, landed by the-wire); no
  warning touches this PRD's footprint. The "clippy clean on the whole
  footprint" claim holds.

## Restoration check (report Findings 1–2)

The wipe-and-restore is real and clean. Byte-diffs of
`prds/lua-interface/probe/src/*` against the landed files:

- `socket.rs`, `context.rs`, `lua.rs`, `main.rs`, `node.rs` → **byte-identical**
  to landed. the-wire's collect (`ab7930e`) touched only `cartridge.rs`,
  `sdk.rs`, `tests/*`, `Cargo.toml`, so those five files could not have lost
  the-wire work; nothing did.
- `cartridge.rs`, `loader.rs` → stale exactly as the report says: the probe
  copy lacks the-wire's stream/subscription machinery and the-ledger's
  `pub(crate) passed_on`. The landed versions are supersets — `11322b7`'s
  diff shows the delta layered on `ab7930e` is only the visibility widening
  of `Link::new/accept/shutdown`, `Remote::over`, and the `config` field
  chain. Nothing the probe originally proved lost a test; the landed probe
  script and the landed `tests/node.rs` cover the same ground.

## Box-by-box

spec01 (6 boxes):

1. **One ask brings the chain up; last line names socket + chain** —
   `tests::node::a_chain_node_calls_its_dependency_and_serves_its_dependents`
   asserts `line["nodes"] == ["echo","tool"]` and a `socket` key, parsed from
   the ask's own stdout. Test-proven.
2. **tool.hello answered by the entry's handler, its need answered one node
   down** — same test, `reply["data"] == {"said":"hello","from":"echo",
   "via":"document"}`. Test-proven.
3. **The composed cartridge serves through the node's socket** — same test,
   `child.note` answers `{"said":"nested","from":"echo","via":"child"}`.
   Test-proven.
4. **An unprovided key refuses at the socket with `` `<key>` is not provided
   ``** — **not test-proven.** The landed test asserts only
   `reply["reply"] == 1` (src/tests/node.rs:198-200); a successful call also
   carries `reply: 1`, so the assert pins nothing about refusal or message.
   The behavior itself is real — I ran the landed binary on a fresh tree:
   `call nowhere` → `{"error": "`nowhere` is not provided", "reply": 3}`, the
   string coming from `Host::call` (src/lua.rs:468) through the socket's
   error envelope (src/socket.rs:150-156) — but the only proof is the probe's
   *printed, unasserted* output plus my rerun. The probe script
   (`probe/write-wire-run.sh`) has no assertion on the text either. The
   correction bullet below closes this with one assert.
5. **The cascade** — test kills echo and asserts `!alive(tool)`
   ("the dependent followed its dependency"); the probe independently
   fails on `kill -0` of the tool's pid. Test- and probe-proven.
6. **Every resolver and baseline test passes** — 97/0 on main, and the diff
   touches `tests/resolver.rs` only for helper visibility (`fn` → `pub fn`).
   Test-proven.

spec02 (4 boxes):

1. **Document config reaches `apply` when the caller names none, in-host and
   as a hosted node** — `tests::node::the_document_carries_the_cartridges_
   own_config` asserts `{"via": "document"}` in-host; the chain test's
   `tool` document carries `config` and its answer carries `via: "document"`
   through `hosted_node`, which passes `Value::Null` (src/main.rs:259) and
   takes the `load_component` fallback (src/lua.rs:246-248). Test-proven on
   both paths.
2. **A caller's own config is laid over** — same test asserts
   `{"via": "caller"}`. Test-proven.
3. **No-`config` documents behave as before** — suite-backed: all 97 tests
   compose documents without the field, and the landed diff touches no
   delivery code except the null-fallback. Suite-proven.
4. **`deny_unknown_fields` still holds; manifest/ledger listings read a
   config document** — `tests::process::invalid_registration_fails_before_
   changing_loaded_fibers` asserts `unknown field \`cmd\`` against the
   `#[serde(deny_unknown_fields)]` struct that now carries `config`
   (src/loader.rs:74,147,192); the probe's `zirkle ledger` listing shows the
   config document read (`tool  provides tool.hello`). Test- and
   probe-proven.

## Findings

1. **Overclaim, report.md:23-24 — the fronting claim is false on the landed
   code.** The report says "The node also fronts its bound needs to its own
   socket callers (`Host::call` falls through to the dependency)." Landed
   `Host::call` (src/lua.rs:457-472) is a bare `peek` of the node's own
   store; the bound needs live in `Host.deps` and are reachable only through
   `LuaCtx::get`. Verified on the landed binary: calling the *bound* need on
   the node's own socket answers `{"error": "`echo` is not provided",
   "reply": 2}`. The claim was pattern-matched from the wire, where fronting
   is real because the sub-host's remotes sit in the store
   (`tests::wire` asserts it at src/tests/wire.rs:165-175) — the chain node's
   never do. No acceptance box requires fronting (the mechanism the specs
   describe is `ctx:get` from Lua, which works), so the record, not the
   code, is what needs correcting.
2. **Bound-need resolution in fibers nested under the node is untested.**
   spec01's opening claims `ctx:get` of a need resolves to the remote "in
   the node's own fiber and in every fiber nested under it". Every
   `ctx:get`-of-a-bound-need in the landed test and probe runs in the
   node's own apply fiber (the test's nested `child` calls a key that is
   *in-store*, not bound). The mechanism is shared `Host.deps` state and
   would work, but the clause is asserted nowhere. Observation, not a box
   violation — no box quotes it.
3. `src/tests/node.rs` ends without a trailing newline (git marks
   `\ No newline at end of file`). Cosmetic.

## Corrections

- `/Users/feb/dev/cartridge/.pearde/prds/lua-interface/report.md`
  ("What stands", chain-link bullet, lines 23-24): delete or rewrite the
  sentence claiming the node fronts its bound needs to its own socket
  callers and that `Host::call` falls through to the dependency — on the
  landed code a bound need called on the node's socket refuses
  `` "`<key>` is not provided" `` (verified against the landed binary), and
  bound needs answer only through `ctx:get` in Lua.
- `/Users/feb/dev/cartridge/src/tests/node.rs` (box 4 of spec01, lines
  190-201): pin the refusal the box names — assert `reply["error"]` equals
  `` "`nowhere` is not provided" `` instead of the `reply: 1`-only assert,
  so the box's message is proven by a test and not only by the probe's
  printed output.

CHANGE — the landed code and every acceptance box hold (97/0 green,
restoration verified clean), but the report's fronting claim is disproven by
the landed `Host::call` and box 4's refusal message is asserted by no test,
so both must be corrected before the record is trusted.