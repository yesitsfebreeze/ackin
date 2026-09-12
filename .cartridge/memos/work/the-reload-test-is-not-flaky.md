---
kind: work
description: "The cartridge reload test passes or fails for a reason, not by luck"
status: open
level: 10
priority: P2
---

# the-reload-test-is-not-flaky

## Outcome

`core/tests/test_development.py::test_cartridge_reload_and_failed_build_keep_the_surface`
is flaky. It was seen failing two different ways and then passing on a rerun of
the same binary, on a lane and on the trunk alike, during the 2026-09-12 work
pass. A flaky gate is worse than a missing one: it teaches every agent and every
person to rerun a red test instead of reading it, which is exactly how a real
failure gets waved through.

Find why it races — the reload path, the build it waits on, or the assertion's
timing — and make the test deterministic, or replace it with one that proves the
same behaviour without the race. Quarantining it is not the outcome; a test
nobody trusts should either be fixed or deleted with its reason recorded.

## Check

- [ ] The test passes 20 consecutive runs on one binary, unmodified.
- [ ] The mechanism of the old flakiness is named in the commit or the memo, not
      just made to disappear.
- [ ] `just test` stays green.
