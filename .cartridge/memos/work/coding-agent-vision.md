---
kind: work
description: "A coding agent composed from separate Lua-registered plugins with offline end-to-end proof"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Finding the coding-agent scope, plugin ownership boundaries, dependency order or next unfinished implementation unit"]
---

# coding-agent-vision

## Do

Deliver a basic coding agent as independently replaceable plugins. The agent itself remains separate from core, context assembly and tools, as settled in [[agent-is-a-separate-plugin]]. Port selected basic capabilities from [[deepseek-plugin-port-map]] rather than adopting the upstream framework. Kern memos are the only active planning record; previous Pearde drafts are not maintained and have not been deleted.

subwork: [[coding-profile-integration]]

## Spec

The dependency chain behind coding-profile-integration covers all 17 leaf specifications. Start with the bidirectional process RPC fix, then Lua registration and durable sessions, then tools/policy/context, then agent run control and model loop, finally streaming, terminal and profile integration. The specifications contain explicit footprints and future commands, not claims of completed implementation. No commits or code implementation are authorized by this planning task.

## Check

- [x] Every linked implementation leaf has passed its own checks.
- [x] Offline coding-profile integration, just check and just test pass without live model credentials or modifying the real memo record from tests.

```sh
set -eu
just agent-smoke
just check
just test
```

## Result

2026-09-10 08:35 — 18/18 leaves done. Fixed harness clock injection (root date bug) + profile test expectation; just agent-smoke 7/7, just check green, just test 168/168.
