---
state: open
origin: requested
priority: 80
complexity: 0
blast-radius:
needs:
  - the-manifest
---

# The ledger

The registry the resolver reads: every cartridge installed on this machine, what
each one provides, what each one needs.

What exists today resolves within a single profile's hand-listed
`.zirkle/<profile>/init.lua`; `deps()` in `src/main.rs` and the loader's
inject-to-provider search walk only that list. A cartridge that is present but
unlisted does not exist.

Replace that: the host scans a cartridge root, reads each manifest, and derives
what is available from what is there. A profile becomes an override on top of the
ledger rather than the manifest of record.

Auto-registration is the requirement. Installing a cartridge is putting it where
the ledger looks. Uninstalling it is taking it away. Neither involves editing a
list, because a list that must be edited is a second place for the truth to live.

The namespacing decision in [[the-manifest]] is **settled**, and it fixes this
PRD's data structure before a spec is written: *hidden until passed on*. The
ledger is therefore a **namespace of subtrees, not a flat table.** An entry is
identified by its path from the root, never by its bare name; two cartridges may
provide the same key without colliding; a key resolves against the asking
cartridge's subtree and walks outward; and a parent that wants an inner key
visible outside re-exports it by name in its own manifest. Removing a parent
removes its whole subtree, which is what makes the auto-registration above
symmetric — putting a tree where the ledger looks installs everything in it,
taking it away uninstalls everything in it.
