---
kind: work
description: Run a cartridge's own tests when its declared version changes, keyed per cartridge and version
status: open
level: 10
---

## Outcome

Every cartridge owns its responsibility and is verified individually: its
registration contract runs at apply, and its tests run automatically when the
cartridge changes. The trigger is a version in the manifest — bumping
`version` in `cartridge.json` re-runs that cartridge's suite; an unchanged
version keeps the recorded result.

## Check

- [ ] `cartridge.json` accepts an optional `version`; a bumped version makes
      the runner re-run that cartridge's tests on the next watch cycle.
- [ ] Only the changed cartridge's tests run; unrelated cartridges are not
      re-verified.
- [ ] The runner records `{cartridge, version, result, evidence}` and serves
      the last result without re-running while the version is unchanged.
- [ ] A failing suite reports per cartridge, naming the cartridge, and the
      rest of the profile keeps running.

## Approach

- A runner service records results per `(cartridge, version)` under
  `.zirkle/` state; the watch/reload path already knows which cartridges
  changed (`replace_changed`), so it hands that set to the runner.
- Per-cartridge suites map to what exists: `cargo test -p <name>` for Rust
  cartridges, `bun test` for `ui`, the Python suite for `tools`, plus the
  declared `selftest`/`integration` contracts that `zirkle --profile tools
  verify` already runs.
- The scoped-gate practice ([[fresh-checkout-gates]], "the gates are scoped
  to the diff") becomes mechanical: the version is the diff marker.

## Context

- [[zirkle-diagnostics-leaks-into-pty-tests]] — test runners must scrub the
  parent environment (`ZIRKLE_DIAGNOSTICS`, `ZIRKLE_PROXY_KEY`,
  `ZIRKLE_MODEL_KEY`) before spawning cartridges, or suites fail on
  inherited state the cartridge never chose.