---
kind: principle
description: Apply when concurrent actors might write to the same file, branch, key, or state object. Eliminate the sharing first; serialize structurally only when one shared writer is a real invariant.
group: architecture
---

# Separate before serializing shared state

When concurrent actors might share mutable state, first ask whether they need
the same mutable object. If not, eliminate the sharing. When sharing is real,
enforce serialization structurally — lockfiles, sequential phases, exclusive
ownership. Instructions and conventions are not concurrency control.

**Why:** Concurrent writes to shared state create races that are intermittent,
hard to reproduce, and expensive to debug.

**The pattern:**

1. **Identify shared mutable state:** files both read and write, branches both
   push to, APIs both define and consume.
2. **Default: eliminate the shared write target.** Do these actors need one
   canonical object, or are they publishing independent facts? Give each actor
   its own owned file, key, branch or directory, and merge only at the read
   boundary. Two workers writing their own field into one `state.json` is
   still shared mutation; two separate state files is not.
3. **Only when one shared write target is a real invariant, serialize access
   structurally** — a lockfile, sequential phases, a single-writer actor, or
   atomic compare-and-swap. Treat "we need a lock" as a smell to check, not as
   the default answer.

The write-time question that avoids this entirely is the concurrency corollary
in [[foundational-thinking]].
