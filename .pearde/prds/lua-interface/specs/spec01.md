---
complexity: 16
footprint:
  - src/main.rs
  - src/lua.rs
  - src/context.rs
  - src/socket.rs
  - src/cartridge.rs
  - src/lib.rs
  - src/tests/node.rs
  - src/tests/mod.rs
  - src/tests/resolver.rs
---

# spec01 — a chain node calls its dependency and serves its dependents over one socket per node

The wire across chain nodes, which the resolver's pass one named open: each
node of a `zirkle up` chain serves its provided keys on a unix socket derived
from the root and its own ledger path (`socket::node_path`), and connects to
the socket its dependency serves (`ZIRKLE_DEP`, set by the dependency when it
re-enters). Every need of the document is bound to a remote over that socket —
the same `Link`/`Remote` pair the process wire speaks, so the reply envelope
decodes identically at both ends — and `ctx:get` of a need resolves to it, in
the node's own fiber and in every fiber nested under it. A need that is bound
refuses nowhere: the author calls it like any other provided key. A key that
is neither provision nor bound need keeps the store's own refusal, and a
dependency's socket going away fails every in-flight call while the stdin pipe
carries the cascade untouched.

This unit stands in the lane (built, not committed — `prds/lua-interface/probe/`
holds the probe copy): the boxes are what an implementer re-verifies and a
later worker must not break. Stands beneath it: the resolver's pass one chain
(`up`/`enter`/`node`, ported into this lane as the base commit `6fe121c`), the
ledger's bindings, and the socket transport the wire settled on.

## Acceptance

- [x] one ask of `zirkle up <tool>` brings a chain of Lua-entry cartridges up
      from the bottom over real processes, and the ask's last line of output
      names the socket the top node serves and the chain it launched
      — `{"nodes":["echo","tool"],"socket":"/tmp/zirkle-241d35e410fa9ca4-feb.sock","up":"tool.hello"}`
- [x] a call of the named tool's key on that socket returns the entry's own
      handler's answer, with the handler's call of a document need answered by
      the dependency one node down
      — `{"data": {"from": "echo", "said": "hello", "via": "document"}, "reply": 1}`
- [x] a cartridge the node composes itself (`ctx:cartridge`) serves through
      the node's socket, and one call of its key reaches down through the node
      to the dependency and back
      — `{"data": {"from": "echo", "said": "nested", "via": "child"}, "reply": 1}`
- [x] a key that is neither the node's provision nor a need the chain bound
      refuses at the socket with "`<key>` is not provided"
      — `{"error": "`nowhere` is not provided", "reply": 1}`
- [x] killing the far end still takes its dependents with it — the sockets
      carry the calls and the stdin pipe carries the cascade, and neither
      depends on the other
      — probe: `the tool went with its dependency (pid 41779 gone)`; test:
      `the dependent followed its dependency`
- [x] every resolver and baseline test still passes unchanged
      — `cargo test --lib`: `test result: ok. 97 passed; 0 failed` (89 at
      spec-writing time; the-wire's landed `tests::wire` adds 8)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/lua-interface
cargo test --lib tests::node::
cargo test --lib
sh /Users/feb/dev/cartridge/.pearde/prds/lua-interface/probe/write-wire-run.sh
```

Closed 2026-09-12 (wren, implementer pass): `tests::node::` — `2 passed;
0 failed; 95 filtered out`; `cargo test --lib` — `97 passed; 0 failed`; the
probe green over real processes: six files written, the ledger reading the
tree, one ask up, two calls answered across the chain, the unprovided key
refused by name, and the dependent gone with its dependency.