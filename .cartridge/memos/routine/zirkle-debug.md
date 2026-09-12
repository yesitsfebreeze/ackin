---
kind: routine
description: Enter, query and leave debug mode on the running cartridge host from the shell — `cartridge debug on`
  taps everything the host puts on its socket into .cartridge/logs/debug.log, `cartridge debug` reports, `cartridge debug
  off` restores the host. Use to collect evidence about what the host actually did while reproducing a
  problem, instead of guessing from a truncated ui.log.
uses:
- usage: '[[run-usage]]'
  when:
  - diagnose why a tool or cartridge misbehaves
  - collect evidence for a hypothesis about the running host
  - see the events the host emitted during a command
  - turn cartridge diagnostics on and off
  tags:
  - debug
  - diagnostics
  - cartridge
  - host
  - evidence
---

## Inputs

Requires the running host: these are socket requests, answered by the `cartridge run`
or `cartridge daemon` that is serving the current profile. Run them from the same
working directory as that host, or pass the same `--profile`. `cartridge socket`
prints the socket they address; `no daemon for ...` means no host is running
there. Risk: danger none — debug mode touches no cartridge, no fiber and no
terminal, so the shell, its cwd and its running program are unaffected.

## Do

| Recipe | Does |
|--------|------|
| `cartridge debug on` | enter debug mode; replies with the log path |
| `cartridge debug` | report whether it is on, the log path and its size |
| `cartridge debug off` | leave; the tap is dropped and the host is back to normal |
| `cartridge status` | every fiber with its state, injections, provides and error |
| `cartridge list` | every cartridge and each injected key resolved to its provider |

The loop is: `cartridge debug on`, reproduce the problem, read the log, `cartridge debug
off`. While on, every message the host puts on its socket — cartridge events,
cartridge errors and fiber transitions — is also appended as one JSON object per
line to `.cartridge/logs/debug.log`, beside the profile. The file is appended to, not
truncated, so note its size before reproducing and read only what came after.

Entering twice is entering once; leaving without having entered is not an error.
Debug mode is answered directly by the host and needs no cartridge, so it still
answers while the host is busy inside a service call.

## Check

`cartridge debug` reports `"on":true` after `on` and `"on":false` after `off`, and
`bytes` grows across the reproduction. The host is still there afterwards:
`cartridge status` lists the same fibers in the same states.

## Failure

`no daemon for <profile>`: no host is serving that profile from this directory —
start one (`just run`) or `cd` to where it runs. A `cartridge debug` that prints
nothing and exits 0 means the host went away mid-request; check it is still
running. `bytes` that does not grow means the host emitted nothing: the activity
you are reproducing is not reaching the socket, so look at the cartridge that
should be emitting rather than at debug mode.
