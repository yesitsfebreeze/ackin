---
vision: One small binary that resolves a tool's dependency chain at runtime and re-enters itself down it, so the running process tree is the dependency tree — and every capability is a cartridge written, tested and swapped on its own.
terminals:
  - lua-interface
  - hot-reload
  - the-sandbox
  - verify-one-cartridge
  - the-telemetry-channels
edges:
  - "the-host-binary -> the-manifest"
  - "the-manifest -> the-ledger"
  - "the-ledger -> the-resolver"
  - "the-resolver -> the-wire"
  - "the-wire -> lua-interface"
  - "the-wire -> hot-reload"
  - "the-resolver -> hot-reload"
  - "the-manifest -> the-sandbox"
  - "the-resolver -> the-sandbox"
  - "the-wire -> verify-one-cartridge"
  - "the-wire -> the-telemetry-channels"
---

# The destination

A cartridge is a program that does one thing and declares two facts about
itself: what it provides, and what it needs. Nothing else. It is written in
whatever language suited the job, it is tested on its own, and it is thrown
away and rewritten when a better version of it exists. The value of the
system is the number of cartridges in it, not the size of what runs them.

What runs them is small enough to disappear. The binary holds no plugins, no
features and no opinions — installed on its own it does nothing at all, and
that is the point. It is installed once. Everything after that is cartridges.

## The mechanism

It is not a supervisor. Nothing sits above the tree owning it.

You name a tool. The binary reads the ledger, walks that tool's dependency
chain to its far end, and spawns the *first* program the chain requires —
not the tool you asked for. That program comes up, and re-enters the binary
for the next tool along the chain. That one comes up and re-enters it again.
The chain assembles itself from the bottom, one re-entry per link, until the
tool you named is running at the top of a tree that was resolved entirely at
runtime, out of whatever the ledger happened to hold at the moment you asked.

The binary is therefore invoked once per node, and each invocation has one
job: resolve a single step and hand off. There is no long-lived coordinator
to lose state, restart, or become the thing everyone waits on.

## What the shape buys

Because a dependency *launches* its dependent, the process tree and the
dependency tree are the same tree. Nothing maintains that correspondence;
it cannot drift.

- **Teardown is free and correctly ordered.** A node exits, and everything
  that needed it goes with it. Uninstalling is the absence of a launch.
- **The channel is the birth pipe.** A node talks to its dependency over the
  connection it was created with. No discovery, no broker, no address.
- **Failure is scoped.** A cartridge that dies takes its dependents and
  nothing else. The rest of the tree never knew it existed.
- **Reload is a subtree.** Replace one node and what stood on it re-resolves
  against the new one; what stood beside it is untouched.
- **The blast radius is declared.** A manifest that names what a cartridge
  needs is also the sandbox policy it runs under, so the contract that
  admits a cartridge is the same contract that contains it.

## What the board adds up to

A tool belt that gets better the more is slotted into it. Every piece small
enough to hold in one head, verified against its own contract before it is
allowed to matter, and reachable by any other piece through one wire that is
the same on every system. Growth costs nothing at the centre, because there
is no centre — only the ledger, and what it can be asked for.
