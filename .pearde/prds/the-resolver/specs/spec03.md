---
complexity: 12
footprint:
  - src/main.rs
  - src/loader.rs
  - src/sdk.rs
  - src/tests/fixtures/chain_fixture.rs
  - Cargo.toml
---

# spec03 — a Lua-entry cartridge is hosted: the binary is the program a chain node runs

The one unit the probe could only stand in for. Every cartridge in the tree
today has `entry: init.lua` and no `binary`, so the resolver's refusal —
"`{path}` declares no binary: the chain has no program to launch" — is what a
real chain hits today, and the mechanism is only provable with a stand-in
fixture. The answer this spec makes stand: a cartridge without a `binary` is
hosted by the binary itself. The `enter` hand-off, instead of refusing, execs
into the host's own node mode: the host process runs the cartridge's Lua
component in-host as the chain node — it re-enters the binary for the rest of
the chain exactly as the fixture does (holds the child with `kill_on_drop`,
watches its stdin for the EOF cascade), and provides its keys the way an
in-host fiber does. A document that declares a `binary` keeps today's
behaviour: that program is the chain node. A `binary` that does not exist is
refused the ask names the path.

What stays open and out of this spec: where a detached node's diagnostics go
(stdio is null — telemetry/wire territory), ready signalling, and
already-running detection.

## Acceptance

- [x] a chain of ordinary Lua-entry cartridges (no `binary` anywhere) comes
      up from the bottom under one `zirkle up`, and `ps` shows the dependency
      tree — the host process is the chain node
      > `a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency`;
        probe: `db pid ppid=1 (hosted)`, `store ppid=db`, `tool ppid=store`
- [x] a hosted Lua node that re-enters for its dependent dies when its
      dependency goes away, and takes its dependents with it — the cascade
      holds through a hosted node, not only a stand-in
      > same test's kill leg: `!alive(store) && !alive(tool)` after `kill db`;
        probe: "dependents still alive after the hosted far end died: 0"
- [x] a document whose `binary` names a program that does not exist refuses
      the ask naming the node and the missing program
      > `a_chain_node_whose_binary_does_not_exist_refuses_the_ask`: stderr
        names `store` and `gone_node`; no marker written (nothing spawned)
- [x] every resolver test in `src/tests/resolver.rs` still passes with
      hosting standing — the fixture's route and the hosted route agree on
      the tree, the cascade, and the refusals
      > `cargo test --lib tests::resolver::`: 12 passed (7 walk/refusal,
        4 end-to-end across both routes, 1 re-entry refusal);
        `cargo test --lib`: 87 passed; `cargo clippy --all-targets` clean

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-resolver
cargo test --lib tests::resolver::
cargo test --lib
sh /Users/feb/dev/cartridge/.pearde/prds/the-resolver/probe/the-walk-is-the-launch.sh
```