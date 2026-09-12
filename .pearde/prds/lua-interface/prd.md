---
state: open
origin: requested
priority: 70
complexity: 0
blast-radius:
needs:
  - the-wire
---

# Lua interface

Lua stays. The user has confirmed it directly: it is the composition and
interface layer and it is not deleted.

It is how something is wired in and tried before it has earned a compiled
implementation — the interface code that makes a cartridge slottable while it is
still being vibed into shape. `mlua` is already a dependency and `src/lua.rs`,
`src/context.rs` and `src/loader.rs` already carry the host bindings, the `ctx`
surface and the entry protocol.

What this PRD settles is the surface a cartridge author writes against now that
the binary re-enters per node rather than composing a profile up front. The old
shape assumed a profile `init.lua` listing entries into one graph. The new shape
starts from a named tool and [[the-ledger]], which is a different thing to write
against even though it runs the same interpreter.

At the end, a new cartridge can be written, wired and run without touching Rust.
