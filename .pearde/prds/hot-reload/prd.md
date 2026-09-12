---
state: open
origin: requested
priority: 65
complexity: 0
blast-radius:
needs:
  - the-resolver
  - the-wire
---

# Hot reload

Replace one node while the tree is running. What stood on it re-resolves against
the replacement; what stood beside it is untouched.

The transaction is already implemented and already proven — `~/dev/sys/core/README.md`
describes it and `src/runtime.rs` and `src/fiber.rs` carry it: prepare the old
cartridge, drain its service calls, freeze its bank, hand the candidate a private
copy, publish only an active candidate presenting the same provided keys,
unfreeze the old bank when the candidate is rejected, and dispose in dependency
order. That logic survives this PRD intact.

What changes is scope. Reload was a profile operation; here it is a subtree
operation. The blast radius of replacing a node is exactly its dependents, and
[[the-resolver]] makes that set knowable from the process tree without any
bookkeeping — the tree already is the answer.

Must not change: `host.on_reload` receiving `true` to prepare and `false` to
cancel, and the rule that a candidate needing an exclusive resource still held by
the old process must fail preparation rather than bind it twice.

At the end, a cartridge is rebuilt and swapped under a running tree, its
dependents follow it, and nothing else notices.
