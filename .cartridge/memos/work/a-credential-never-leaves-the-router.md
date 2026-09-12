---
kind: work
description: "A credential is readable by the router and by nothing else that writes"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Changing credential storage, the proxy's auth handling, logging or transcript writing"]
---

# a-credential-never-leaves-the-router

## Outcome

The router reads credentials from its configured `config_dir` and they appear
nowhere else: not in a transcript, not in the memo record, not in a log line,
not in an error message, and not in the events the UI polls.

A secret that leaks into a transcript is not recoverable by deleting it later —
the transcript is durable and the record is committed — so this wants a test
that would catch it the first time rather than a review that might.

## Check

- [x] `test_the_credential_reaches_the_provider_and_nothing_else`: the run
      completes and the provider that answered saw `Bearer <credential>` on the
      wire — the secret itself, not `alpha-secret`, the name the config refers
      to it by.
- [x] The same test walks every file the run left anywhere under its root,
      excepting the credential store itself, and asserts the string appears in
      none of them. Scanning the whole tree rather than named directories means
      a future cartridge that writes somewhere new is covered without anyone
      remembering to add it.
- [x] `test_an_authentication_failure_is_actionable_without_quoting_the_key`:
      a 401 from every provider surfaces the route and the provider's own
      message, and neither the error nor any file holds the credential.
- [x] `test_the_proxy_refuses_a_wrong_key_without_naming_the_right_one`: the
      proxy answers `401 invalid proxy key`, naming neither the key it wanted
      nor the upstream credential; the right key is accepted and answered, so
      the refusal was the key and not the request.

## Approach

Built on the harness from [[the-router-ranks-and-recovers]], with two additions:
a provider that answers 401, and a live proxy.

## Result

`zirkle run` disposes after one call, so the proxy's listener exists only while a
daemon is up. The last test starts `zirkle daemon`, polls `zirkle call router
'{"op":"session"}'` until it answers, and talks to the address it hands back.
That is also the shape [[launch-runs-an-external-agent-against-the-proxy]] and
[[concurrent-sessions-each-own-a-router]] will need.

The containment check is deliberately a whole-tree scan rather than a list of
directories. A leak is only interesting where nobody thought to look.
