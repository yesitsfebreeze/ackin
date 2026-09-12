---
kind: work
description: "The router picks a provider by rank and moves the run to another when one fails"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the catalog, ranking, health or recovery in builtin/router"]
---

# the-router-ranks-and-recovers

## Outcome

A run asks for a model and the router picks a provider for it by the catalog's
ranking. When that provider fails mid-request, the run moves to the next
candidate and completes, without replaying work the agent already did.

Nothing exercises this today: `core/tests/profile.rs` replaces the router with a
Lua fake, so `builtin/router`'s `catalog`, `frontier` and `health` have no test
above their own units. The harness this needs — two fake upstream providers over
loopback, one of them scripted to fail — is the piece to build first, and the
three sibling memos will want it too.

## Check

- [x] `test_the_ranking_offers_every_provider_of_a_model_in_order`: `routes`
      names both candidates in a definite order, and with both answering the
      request goes to the first while the second is never asked.
- [x] `test_a_provider_that_dies_mid_response_hands_the_run_over`: a provider
      that sends its headers and one chunk and then drops the connection hands
      the run to the next, which returns one answer — the half-sent one leaves
      no fragment behind.
- [x] `test_a_failed_provider_is_demoted_and_returns_once_it_recovers`: the
      failure is recorded in `status.health` against that provider; while it is
      down the run goes straight to the other; it is a candidate again once it
      answers.
- [x] `test_when_every_provider_fails_the_error_names_what_was_tried`: the run
      ends with `demo-model@<provider>` and the upstream's own message, having
      asked each exactly once.

## Approach

`core/tests/test_router.py`. The router's catalog and proxy are private to its
binary, so the test drives `zirkle run router` against a profile pointing at two
fake upstreams, each an `http.server` in the test process.

## Result

Landed with the shared harness the sibling memos need;
[[a-credential-never-leaves-the-router]] already borrows it for its first box.

Four things the probe established, worth having written down:

- A provider's `key` names a credential in `config_dir/credentials.json`; it does
  not hold one.
- Declaring `providers` adds to the built-in list rather than replacing it, so a
  test sees `chatgpt` and `github-copilot` attempt inventory and fail. Harmless,
  but it is noise in the log.
- Inventory for an ordinary provider is `GET {base}/models` returning
  `{"data":[{"id","context_length"}]}`.
- The router asks upstream to stream. A fake that answers with a whole
  completion gets `stream ended without output`, which reads like a router bug
  and is not one.

The fake upstreams must be threaded. A single-threaded `HTTPServer` deadlocks
the run it is meant to answer: the router opens more than one connection, and
the second waits on a handler that is still holding the first. That showed up
first as a test that passed alone and hung in the full suite, which is the
shape most of this session's flakes took.

Which of two equal providers leads is not fixed across runs — it moves with the
health the router has recorded. The tests follow what `routes` reports rather
than asserting a winner, which is also the honest statement of the guarantee:
the order is the router's to decide and the run follows it.
