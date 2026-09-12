---
kind: work
description: "Cartridges declare two proof obligations at registration: a functionality self-check and an integration check, enforced by the gates"
status: done
owner: "sys-opus-2026-09-12/implementer-cartridges-prove-themselves-at-registration"
level: 10
priority: P1
estimate: 2d
actual: 4h
---

# cartridges-prove-themselves-at-registration

## Outcome

A cartridge manifest can declare two contract obligations at registration, carried through the core event system (`cartridge.rs`, `runtime.rs`, `fiber.rs`): a functionality self-check (proves its own behavior, runtime-unit-test style) and an integration check (proves it wires correctly with what it provides, injects, or depends on). Both are named routines the cartridge already exposes over its existing SDK wire, not a new proof language or dependent-type system — scoped down from lean-lang.org's proofs-as-programs idea to a lightweight declarative contract. `zirkle run`, `just check`, and `just test` invoke and gate on them the same way they gate on existing checks. A cartridge with no declared contract behaves exactly as today; this is opt-in, not a new requirement on every cartridge.

## Check

- [x] A cartridge manifest can name a `selftest` and an `integration` routine; omitting either keeps current behavior unchanged.
- [x] The gate entrypoint invokes both routines after apply and fails closed on a nonzero/error result, naming the failing cartridge and routine in its output.
- [x] The integration check observes at least one other cartridge's provided service (a real dependency, not a mock), proving the wiring rather than only the cartridge's own logic.
- [x] `just check`/`just test` runs contracts for every cartridge that declares one; a deliberately broken self-check or integration check fails the gate and a working one passes.
- [x] One real `builtin/*/README.md` documents its contract as the pattern for future cartridges.

## Approach

Reuse the existing SDK call wire (`sdk.rs`) rather than adding a new RPC shape — a contract call is an ordinary service call the cartridge already knows how to serve. Land on one fixture cartridge first (reuse the one from [[the-agent-can-extend-and-verify-a-cartridge]] if still available) before touching real builtins.

## What landed

`loader::Cartridge` gained two optional manifest fields, `selftest` and
`integration`, each naming a key the cartridge already provides.
`Host::verify` loads the profile, waits for every fiber to leave `Loading`,
calls each declared key with null args after apply, disposes the instances and
returns `(ran, failures)`; a contract fails when its call errors or returns
`false`. `zirkle --profile <name> verify` is the gate entrypoint and exits 1 on
any failure. `just test` runs `zirkle --profile tools verify`.

`builtin/harness` provides `harness.selftest` (renders a context from its own
configuration — frame and one projected message — with no dependency involved)
and `harness.integration` (calls the injected `memo` service for the system
template every request is built from). `builtin/tools` provides
`tools.selftest` (the workspace root resolves and every `builtin/*/cartridge.json`
reads as the bundler reads it). The pattern is documented under
"Registration contracts" in `builtin/harness/README.md`.

## Evidence

Boxes 1 and 2, fixture cartridges over the real runtime
(`core/tests/contracts.rs`), `cargo test -p zirkle --lib tests::contracts`:

    test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
    test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
    test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
    test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
    test tests::contracts::one_obligation_may_be_declared_alone ... ok
    test result: ok. 5 passed; 0 failed

Box 2 on the gate entrypoint, `zirkle --profile tools verify` with the manifest
pointed at a key the cartridge does not provide:

    tools selftest `tools.broken`: `tools.broken` is not provided
    exit=1

Box 3, the real composed profile with the real cartridge binaries
(`cargo test -p zirkle --lib declared_contracts`): the harness's integration
check reaches the memo cartridge over the process wire, and with `memo`
replaced by a stub that errors it is the only obligation that fails —

    test tests::profile::declared_contracts_prove_the_real_cartridges ... ok
    (asserts failures == ["harness integration `harness.integration`: … record is down"])

Box 4, `zirkle --profile tools verify` against a deliberately broken
`tools.selftest` body and then the working one:

    tools selftest `tools.selftest`: deliberately broken self-check
    exit=1
    1 contracts passed

Gates in the lane: `just check` exit 0; `just test` exit 0, with
`235 tests run: 235 passed` from nextest and the gate's own line —

    1 contracts passed
