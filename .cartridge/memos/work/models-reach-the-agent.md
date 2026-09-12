---
kind: work
description: "The router gets a model to the agent, and recovers when one fails"
status: done
subwork:
  - "[[the-router-ranks-and-recovers]]"
  - "[[a-credential-never-leaves-the-router]]"
  - "[[launch-runs-an-external-agent-against-the-proxy]]"
  - "[[concurrent-sessions-each-own-a-router]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Changing providers, credentials, the catalog, ranking, recovery, the HTTP proxy or launch"]
---

# models-reach-the-agent

## Outcome

The agent asks for a model and gets one. The router owns providers, credentials,
the catalog and its ranking, and it recovers from a failed or truncated request
without losing the run or replaying work already done. It serves an HTTP proxy
on an OS-assigned loopback port so external agents can be launched against the
same credentials and policy.

Scope is the router cartridge and the proxy. What the agent does with the tokens
belongs to [[the-inline-agent]].

## Check

- [x] Every child work memo listed in `subwork:` is done.

Each box of the original Check became one child, because each needs the same
harness and then asks a different question of it:
[[the-router-ranks-and-recovers]], [[a-credential-never-leaves-the-router]],
[[launch-runs-an-external-agent-against-the-proxy]] and
[[concurrent-sessions-each-own-a-router]].

## Approach

Split rather than delivered. Probing found that nothing exercises `builtin/router`
above its own units: `core/tests/profile.rs` replaces the router with a Lua fake,
so the catalog, ranking, health and recovery paths have no test that runs them
together, and neither does `zirkle launch` or the proxy's key handling.

The missing piece is shared: a fake upstream provider over loopback that can be
scripted to succeed, to fail mid-response, or to reject a key. All four children
need it, and the first to be picked up should build it.

`the-router-ranks-and-recovers` is the one to start with — it needs the harness
in its fullest form, and the other three reuse it.

## Result

All four children are done, and `core/tests/test_router.py` is what they left
behind: fake providers in processes of their own, a live proxy through a daemon,
and a stand-in agent launched against it. The router now has a test above its
own units for the things a run depends on — ranking, recovery, health,
credential containment, launch and concurrency.
