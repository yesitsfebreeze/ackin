---
kind: work
description: "The agent can extend a cartridge and prove the extension works"
status: active
owner: "sys-work-2026-09-12/analyst-the-agent-can-extend-and-verify-a-cartridge"
level: 10
priority: P1
estimate: 2d
needs:
  - "[[the-agent-can-discover-its-own-program]]"
  - "[[debug-mode-correlates-a-terminal-turn]]"
---

# The agent can extend and verify a cartridge

## Outcome

Using the program map and debug workflow, the agent can implement a bounded
cartridge change through the visible shell, run the relevant checks, load the
extension and verify its behavior. The resulting capability and its usage memo
become discoverable to subsequent turns. Self-extension is an ordinary tested
change to the program and its record.

## Check

- [ ] A reproducible local scenario starts from a requested capability, resolves
      the appropriate cartridge recipe, edits a fixture cartridge, runs its
      checks, reloads it and verifies the new behavior through real readback.
- [ ] The scenario updates the cartridge's usage/test memos; a subsequent turn
      discovers and uses the new capability through shell and memo without a
      manually expanded system prompt or a new specialized agent tool.
- [ ] A deliberately broken candidate fails its check or reload and leaves the
      working cartridge available. The agent observes the failure, corrects or
      reverts its candidate, and verifies recovery.
- [ ] The shell/editor session survives cartridge replacement; the transcript
      records intent, edits, test outcomes and before/after revisions. Core
      changes are explicitly identified as requiring a host restart.

## Approach

Start with one fixture cartridge and a temporary profile/session using the
existing build, test and replacement machinery. Persist only the reusable
recipe, rationale and verification evidence in the record; keep detailed command
output in the transcript. Exercise the agent loop end to end with a deterministic
local provider fixture so the regression gate needs no paid model credentials.
