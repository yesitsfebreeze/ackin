---
kind: work
description: "The terminal tests fork a PTY from a process that is not reliably single-threaded"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["A python terminal test hangs or fails only in the full suite, or when adding a test that starts threads or subprocesses"]
---

# the-terminal-tests-fork-from-a-threaded-process

## Outcome

`python3 -m unittest discover -s core/tests` runs without
`DeprecationWarning: This process is multi-threaded, use of forkpty() may lead
to deadlocks in the child`.

`core/tests/test_shell.py` and `core/tests/test_development.py` call
`pty.fork()`, and unittest runs every file in one process. Forking a
multi-threaded process can deadlock the child, and a deadlocked child here is a
test that passes alone and hangs in the suite — which is what
`core/tests/test_router.py` did until its fake providers moved from threads into
subprocesses.

That move made the failures rare but did not remove them: `just all` still
failed intermittently afterwards, always in a terminal test, always passing on a
rerun. Rare is the worst state for this — it trains a reader to rerun rather than
to look.

## Approach

What the bisection showed:

- `test_shell.py` alone does not warn.
- Nor does any pair: `test_ui` + `test_shell`, `test_development` + `test_shell`,
  or `test_router` + `test_shell`.
- All four together do warn, during `test_shell`.
- No test leaves a thread behind: `threading.active_count()` is 1 after each
  file runs to completion.

So the thread is transient, belonging to something still finishing when the next
file begins — and the condition under which no file warns is simply being alone
in its process. `just test` now runs one process per file rather than hunting
the thread, which removes the hazard instead of narrowing it.

## Check

- [x] `just test` emits no forkpty warning: each of the four files runs in a
      process of its own, and alone none of them warns.
- [x] `just all` green on three consecutive runs.
- [x] The condition is named here and in the justfile, beside the loop that
      avoids it, so the next person adding a terminal test sees why.

## Result

`just test` runs `python3 -m unittest discover` once per file in
`core/tests/test_*.py` instead of once over all of them.

The thread was never identified, and this does not identify it — it removes the
condition instead. That is the right trade here: the cost is four interpreter
starts, and what it buys is a suite that cannot be poisoned by one file leaving
work running when the next one forks. A future test that needs threads no longer
has to know about `pty.fork()` in a sibling file.
