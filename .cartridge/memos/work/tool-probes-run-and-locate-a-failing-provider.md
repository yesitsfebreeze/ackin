---
kind: work
description: "Contract probes run against disposable fixtures, and an injected failure is traced to its provider and failing check"
status: open
level: 11
priority: P1
estimate: 2d
needs:
  - "[[every-enabled-tool-ships-a-contract-probe]]"
---

# Tool probes run and locate a failing provider

## Outcome

Discovered probes actually execute, and their results are strong enough to
diagnose with.

The end-to-end fixture exercises the two tools the default profile enables:
`tool.shell` command submission, interactive input and screen readback, and
`tool.memo` resolve, read and write against a temporary record. Every
state-changing probe runs against a disposable fixture, never the user's record
or the user's shell session. A probe passes only on observed behaviour — the
command's output, the memo read back — never on a successful dispatch.

Then the loop closes: with a service or tool deliberately made to fail, the
agent inspects the evidence, names the provider and the failing check, and
reruns the probe after the fix to see it pass on the same binary. The recorded
report separates **passed**, **failed** and **unverified** capabilities rather
than reducing them to one status.

Two facts the analyst probe established that this work has to handle. Calling an
agent tool over the socket is refused today — `zirkle call tool.shell …` answers
`invocation context required` and `zirkle call memo …` answers `trusted memo cwd
required` — so a probe has to supply the invocation context and a trusted record
path the way the agent's own dispatch does. And the loop should be provable
without a model: use a deterministic local provider, as the audit
[[runtime-audit-2026-09-12]] and the existing offline profile smoke
(`just agent-smoke`) already do.

Scope: running probes and reporting them. Discovering what to run is
[[every-enabled-tool-ships-a-contract-probe]]; the evidence channel the report
cites is [[a-turn-carries-one-id-through-host-lua-and-bun]].
