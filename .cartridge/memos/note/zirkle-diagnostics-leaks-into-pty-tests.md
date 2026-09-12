---
kind: note
description: The launch diagnostic sink is set in-process, never through ZIRKLE_DIAGNOSTICS, because every child inherits the environment
date: "2026-09-12"
---

`zirkle launch` moves diagnostics off the terminal so the launched agent owns
the screen. The first version did it with `std::env::set_var`, and every child
inherited it: an agent that ran `just test` from inside a launched session gave
each nested `zirkle` the parent's `ZIRKLE_DIAGNOSTICS=.zirkle/logs/launch.jsonl`.
The path is relative, so it resolved inside each test's temp root, `ui.log`
stayed empty, and `test_cartridge_reload_and_failed_build_keep_the_surface`
(core/tests/test_development.py) waited 60 s for a `"running"` line it would
never see.

`turn::divert` now sets the sink in this process only (core/turn.rs). Nothing
about the choice reaches the environment, so no child can inherit it.
`ZIRKLE_PROXY_KEY` is still exported on purpose — the launched agent
authenticates to the proxy with it.

The rule: a process-wide default that a child must not share belongs in the
process, not in its environment.
