---
kind: work
description: "Every enabled tool is one command: discovered, described and called without a hand-written envelope"
status: active
owner: "sys-38/analyst-every-tool-is-one-command"
level: 10
priority: P2
estimate: 1d
---

# every-tool-is-one-command

## Outcome

A person or a script names a tool and gets its result. Discovery and the call
come from the profile, not from knowledge of the wire: the command lists the
`tool.*` keys the loaded profile provides, prints what each one takes from its
own `describe`, and calls one with nothing but that tool's own input. The
session, run, call and cwd of the tool envelope are the command's to supply.

Today the same work is `zirkle run tool.memo
'{"op":"call","context":{"session":"s","run":"r","call":"c","cwd":"…"},"input":{"op":"list"}}'`,
and there is no way to ask which tools exist. That envelope is the wire between
the proxy and a tool; a caller at the shell should not be writing it.

Policy is not bypassed: a call goes through the same `policy` decision the
inline agent's calls go through, and a denial is reported as a denial.
`--yolo` keeps the meaning it already has on `zirkle run`.

The MCP server under this terminal is built on this surface, so its output
has to be machine-readable as well as readable.

## Check

- [ ] A listing command prints the enabled tools of the profile with each
      one's `describe` name and description; adding or removing a key in
      `.zirkle/default/init.lua` changes that listing with no Rust change.
- [ ] Calling a tool takes only that tool's own input JSON — verified by
      `zirkle tool memo '{"op":"list","kind":"work"}'` returning the same
      content as the hand-written `zirkle run tool.memo` form.
- [ ] A successful call prints the tool's content on stdout and exits 0;
      a tool result with `error:true` prints to stderr and exits nonzero.
- [ ] A tool the policy denies fails with the policy's reason, and the same
      call under `--yolo` succeeds.
- [ ] An unknown tool name fails naming the tools that exist, and never falls
      through to an arbitrary service lookup.
- [ ] `just check` and `just test` pass.
