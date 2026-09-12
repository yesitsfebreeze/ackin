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

Depends on the namespacing decision in [[the-manifest]]: a flat ledger and a
ledger of subtrees are different data structures.
