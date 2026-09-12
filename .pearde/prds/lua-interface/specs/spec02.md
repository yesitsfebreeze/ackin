---
complexity: 5
footprint:
  - src/loader.rs
  - src/lua.rs
  - src/tests/node.rs
---

# spec02 — the document carries the cartridge's own config

The ledger shape has no profile `config.lua` to lay fields on at composition
time, so the document is where an author's configuration travels:
`cartridge.json` takes a `config` value (`loader::Cartridge`), `resolve` carries
it on `Declared`, and `load_component` delivers it to `apply` whenever the
caller names no config of its own — a caller that names one lays it over the
document's, as the profile's override always did. The rule holds on every path
that loads a component: the in-host profile graph, `ctx:cartridge` composition,
and the hosted chain node, which passes null and so receives exactly what the
document wrote.

This unit stands in the lane (built, not committed): the boxes are what an
implementer re-verifies. Stands beneath it: the document format's standing
declarations (`provide`, `needs`, `export`, `grant`, `binary`, `selftest`,
`integration`), which this field rides beside unchanged.

## Acceptance

- [x] a cartridge whose document carries `config` and whose caller names none
      receives exactly the document's config in `apply`, in-host and as a
      hosted chain node
      — test: `next_event(&mut rx, "via") == {"via": "document"}` in-host;
      probe: `{"data": {"from": "echo", "said": "hello", "via": "document"}, "reply": 1}`
- [x] a caller that names its own config is laid over the document's, not
      refused — test: the caller's component answers `{"via": "caller"}`
- [x] a document that carries no `config` behaves as before: `apply` receives
      the caller's config, or null
      — `cargo test --lib` 97 passed, 0 failed: every baseline cartridge
      (no `config` field) composes unchanged
- [x] a document with an unknown field is still refused (`deny_unknown_fields`
      holds), and the manifest listing and ledger listing still read a
      document that carries `config`
      — `cargo test --lib tests::manifest`: `15 passed; 0 failed`; probe
      ledger listing reads the config document: `tool  provides tool.hello`

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/lua-interface
cargo test --lib tests::node::the_document_carries_the_cartridges_own_config
cargo test --lib tests::manifest
```

Closed 2026-09-12 (wren, implementer pass): `tests::node::` — `2 passed;
0 failed; 95 filtered out`; `tests::manifest` — `15 passed; 0 failed;
82 filtered out`; the chain-node test's `tool.hello` answer carries the
document's `via` field, proving the node path delivers the document's config.