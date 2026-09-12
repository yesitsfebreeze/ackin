---
kind: work
description: "Two foreground sessions each own a router and do not collide"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the router's listen configuration or anything else a second concurrent session would contend for"]
---

# concurrent-sessions-each-own-a-router

## Outcome

Two `just run` sessions work at the same time. The default profile listens on
`127.0.0.1:0` for exactly this reason — the comment in `.zirkle/default/config.lua`
says each foreground chat owns a router and passes its assigned address to
launched clients — but nothing proves a second session starts, and the failure
mode of a fixed port is a second session that dies on boot.

## Check

- [x] `test_two_sessions_each_own_a_router_and_outlive_each_other`: two hosts on
      the same profile text both answer `router base` at the same time.
- [x] The same test: their addresses differ, both are loopback, and neither is
      the `:4141` the default would have used before `127.0.0.1:0`.
- [x] `test_two_sessions_of_one_host_keep_separate_transcripts`: two sessions in
      one host open a buffer under the same name `transcript` and get different
      buffers, each holding only its own work.
- [x] The first test terminates one host and then asks the other for its base
      again, which it still answers.

## Approach

`core/tests/test_router.py`, class `ConcurrentRouters`, on the daemon pattern
[[a-credential-never-leaves-the-router]] established.

## Result

The two halves of this memo turned out to be different questions. Two *hosts*
cannot share one directory — a daemon owns the socket there — so concurrency is
two roots running the same profile, which is what two checkouts or two
workspaces actually do. Session isolation is a question for one host with two
sessions in it, and belongs to the sessions cartridge rather than the router.
The tests are split along that line.

Session isolation is asserted on the buffers themselves rather than through a
session's `transcript` field, which is null until something sets it: opening a
buffer named `transcript` does not fill it in.
