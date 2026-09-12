---
state: open
origin: requested
priority: 75
complexity: 0
blast-radius:
needs:
  - the-resolver
---

# The wire

How a node talks to the dependency that launched it.

The channel is the connection it was born with: inherited pipes carrying JSON
lines on stdin and stdout, the protocol already specified in the module header
of `src/cartridge.rs` and implemented on the cartridge side in `src/sdk.rs`. No
discovery, no broker, no address, no registry lookup at call time. A node that
needs its dependency is already holding it.

The existing protocol is very nearly symmetric — both halves can `call` and
both can `reply`. That near-symmetry is what makes recursion possible without
inventing a second protocol: a cartridge that speaks the host half of this wire
to children of its own **is** a sub-host, and a tree can nest without the binary
above it knowing that it nested. Closing the remaining asymmetry is this PRD's
work.

In scope: whether the Unix socket in `src/socket.rs` stays the only transport.
Socket-only is correct for POSIX and rules out both cross-machine cartridges and
Windows. A decision either way belongs in a memo.

At the end, a cartridge can host cartridges over the same wire it is itself
hosted on.
