---
kind: decision
description: "Cordis is research reference for this repo, not an adopted framework or dependency"
status: accepted
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking whether to install, vendor, or build on Cordis"]
---

# cordis-is-reference-only

## Decision
Cordis (the framework and its paper, [[cordis-paper]] and [[cordis-repo]])
is read for its ideas. Nothing here installs it, vendors its source, or
ports its API.

## Why
The repo owner said so when this record was set up: "cordis is just
reference material". The repository also marks its own API unstable
(4.0.0-rc.10), so building on it now would buy churn.

## Consequences
Findings about Cordis land as `kind: research` memos, one claim each, each
separating what the paper claims from what the pinned source does. A change
of mind is a new decision naming this one.
