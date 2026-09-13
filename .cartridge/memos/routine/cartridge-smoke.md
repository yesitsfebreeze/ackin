---
kind: routine
description: Exercise real policy, proxy authentication and MCP discovery with isolated local fixtures and no model requests.
---

# Check protocol entry points

Build the required owners, then run `just smoke all`, `just smoke policy`,
`just smoke proxy`, or `just smoke mcp`. The maintained fixtures live in the
common tests folder. Every test uses a disposable profile and owns its child
processes; no current profile, credentials or store is modified.

```just
set positional-arguments
test target="all":
    @CARTRIDGE_SMOKE="$1" bun test "$MEMO_OWNER_ROOT/.cartridge/tests/integration/smoke.test.ts"
```

A passing fixture proves the named protocol and authentication behavior, not
model quality or production readiness. Missing binaries fail explicitly.
