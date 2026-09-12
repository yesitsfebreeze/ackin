---
complexity: 3
footprint:
  - .pearde/prds/the-build-signal/probe
---

# spec01 — the build signal, proved on this host and left runnable

The fault this PRD was opened against is gone: `syspolicyd` pid 498 no longer
exists, `launchd` respawned the daemon as pid 93109 at 16:11:51 on 2026-09-12,
and it sits at 0.2% CPU. Every stage that used to hang now completes. This unit
holds the proof in one bounded command so the next worker, and every acceptance
box on this board that names a compiler, rests on a check rather than on a
claim in a report.

What already stands: `probe/build-signal.sh` is written and green, and it
printed `signal=live` with 53 tests passing against the lane worktree.
`probe/daemons.log` holds the daemon evidence. What is left: run the gate
against the repo root rather than the lane, write what it printed into
`probe/signal.log`, and hand the board back a compiler it can name.

Do not tick a box belonging to another PRD from here. The restored compiler
makes [[the-host-binary]]'s 8 open boxes runnable; running them is that PRD's
work, not this one's.

## Acceptance

- [x] `sh .pearde/prds/the-build-signal/probe/build-signal.sh` exits 0 and its last line is `signal=live`
- [x] every one of its four stages prints `OK` — none prints `HUNG`, `FAILED` or `missing`
- [x] the `cargo-test` stage reports a passed count above zero, so the test binaries linked and ran rather than merely compiling
- [x] no stage is reached by waiting: each is capped, and a capped stage that expires prints `HUNG` and fails the run
- [x] `ps -p 498` prints no process, and the live `/usr/libexec/syspolicyd` is below 5% CPU
- [x] the run's output is written to `.pearde/prds/the-build-signal/probe/signal.log`
- [x] nothing outside `.pearde/prds/the-build-signal/` is modified — `git status --porcelain` on the repo names no file this unit wrote

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge
sh .pearde/prds/the-build-signal/probe/build-signal.sh . 2>&1 \
  | tee .pearde/prds/the-build-signal/probe/signal.log
tail -1 .pearde/prds/the-build-signal/probe/signal.log   # must be: signal=live
ps -p 498 > /dev/null 2>&1 && { echo "pid 498 STILL ALIVE"; exit 1; } || echo "pid 498 gone"
ps -o %cpu,comm -p "$(pgrep -x syspolicyd | head -1)"
git status --porcelain
```
