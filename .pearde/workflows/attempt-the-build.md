---
atomic: attempt-the-build
subject: a rename applied by string match across eight files is a guess until a compiler reads it
date: 2026-09-12
updated: 2026-09-12
runs: 4
tags:
  - atomic
---

## Do

1. Run the project's build over every target, not just the library — for a
   Rust crate that means the binaries link and `cargo test` runs, not only
   `cargo build` or `cargo check`.
2. Clear any stale lock holder the build reports. If the build reports
   nothing at all, check for parked processes from previous attempts
   (`ps -eo pid,stat,time,comm | grep -E 'rustc|cargo'`) and kill them before
   re-running — a hung toolchain leaves children that no message names.
3. Fix what the compiler reports and run it again.

## Done when

- The build prints its success line and exits 0, or it has produced a
  diagnostic that names a file and a line, or it has been shown to hang
  rather than fail — in which case go to step 6 and do not re-run it.

## Fails when

- The build produces no output at all and no process is burning CPU. That is not this atomic's failure and re-running it is waste — it is step 6's whole subject, and the route goes there.
- A box is ticked on a build that was started and never finished. An unfinished build is not a passed one.
- A build that finishes in under a second on a tree that was just edited is
  reported as passed without checking for a compiler cache. Where
  `rustc-wrapper` is set in `~/.cargo/config.toml`, a full recompile can
  report 0.5s on a cache hit. Force the unit with `touch` on the crate root
  and confirm the `Compiling <crate>` line appears, or the success line is
  about a build that never happened.
