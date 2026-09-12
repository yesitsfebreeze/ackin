---
kind: work
description: "zirkle launch runs an external agent against the proxy"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Changing zirkle launch, the proxy listener, or the environment handed to a launched agent"]
---

# launch-runs-an-external-agent-against-the-proxy

## Outcome

`zirkle launch <agent>` starts the proxy on an OS-assigned loopback port, runs the
named agent against it, and exits with that agent's status. The agent reaches
the same models under the same policy as the inline agent, without being handed
the credentials themselves.

## Check

- [x] `test_a_launched_agent_reaches_the_models_through_the_proxy`: a `probe`
      agent declared in `router.agents.launch` receives `PROBE_BASE` and
      `PROBE_KEY` from the environment template and posts a completion to that
      base.
- [x] The same test: the answer is the fake provider's own words, the upstream
      recorded the completion, and the provider's credential appears nowhere in
      what the agent saw — it was given a proxy key, not the key behind it.
- [x] `test_launch_exits_with_the_agent_status`: exit 0 and exit 3 both come
      back as `zirkle launch`'s own status.
- [x] `test_two_launches_at_once_get_their_own_ports`: two launches in parallel
      both succeed with different bases, neither of them `:4141`.

## Approach

`core/tests/test_router.py`, class `LaunchedAgent`. The stand-in agent is a shell
script on `PATH`; `agents.launch.probe` gives it the `{base}` and `{key}`
substitutions the real entries use.

## Result

Two things a later reader will otherwise rediscover the hard way.

`zirkle launch` needs the whole `proxy` profile, not just the router: the proxy
waits on the harness, and the harness refuses a null config with `invalid type:
null, expected struct Config`. The test copies the repository's own
`.zirkle/proxy/init.lua` and supplies its own config beside it.

A fresh `data_dir` has an empty shelf, and an unsynced router answers `model not
on the shelf; run a sync` to everything, which reads like a routing bug. The
test syncs once in setup.

The parallel-launch case runs two processes rather than two threads, and the
fake providers this file uses run in processes of their own, because
`test_shell.py` shares this process and forks a PTY. What is left of that hazard
is [[the-terminal-tests-fork-from-a-threaded-process]].
