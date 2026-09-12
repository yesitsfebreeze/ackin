---
state: open
origin: requested
priority: 55
complexity: 0
blast-radius:
needs:
  - the-wire
---

# Verify one cartridge

Each piece verified on its own, against its own contract, before it is allowed
to matter.

`zirkle verify` already loads a profile and runs every contract its cartridges
declare — the `Command::Verify` arm in `src/main.rs`. What this PRD makes true is
that verification is meaningful for a single cartridge in isolation: you can test
the thing you just wrote without assembling a system around it, because
[[the-ledger]] can resolve its dependencies and [[the-manifest]] states its
contract.

This is what closes the loop the whole design exists for. Write a cartridge,
verify it alone, slot it in. The tool belt gets better because each addition is
cheap to trust, not because anyone audited the whole.

At the end, one cartridge is verifiable without the rest of the tree, and a
failing contract keeps it out.
