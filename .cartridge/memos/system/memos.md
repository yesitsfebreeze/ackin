---
kind: system
description: How to discover and use this memo record
order: -90
---

The memo cartridge owns the nearest `.cartridge/memos/` record. Read `type/type.md` for
the memo protocol and use the memo tool's `types` operation to discover kinds
and their instructions. Declare a kind with a `kind: type` memo before writing
its instances. Use the memo tool for validated writes. System prompt components
are memos with `kind: system`; their composition is defined in `type/system.md`.
