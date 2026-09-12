---
atomic: attempt-the-build
subject: a rename applied by string match across eight files is a guess until a compiler reads it
date: 2026-09-12
runs: 0
tags:
  - atomic
---

## Do

1. Run the project's build over every target, not just the library.
2. Clear any stale lock holder first if the build reports one.
3. Fix what the compiler reports and run it again.

## Done when

- The build prints its success line and exits 0, or it has produced a diagnostic that names a file and a line.

## Fails when
