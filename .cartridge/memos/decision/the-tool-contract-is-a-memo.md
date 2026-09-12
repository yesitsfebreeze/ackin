---
kind: decision
description: A tool's contract is a memo — event-typed by kind, executable from inline just-syntax blocks, one memo serving every reader
status: accepted
date: "2026-09-12"
decided_by: Stefan
---

# the-tool-contract-is-a-memo

## Choice

The base system's tool surface follows the justdown shape
(github.com/yesitsfebreeze/justdown): a tool is *provided* as a contract, and
the contract is a memo. The memo's frontmatter declares the tool's type —
one-shot command, watcher, or whatever event shape it serves — and its body
carries fenced code blocks in justfile syntax that the system executes, so
cross-platform commands are written inline in the memo rather than in a
separate implementation kept in sync with docs. One memo serves every reader,
the jd lenses: humans read the prose, the index reads the frontmatter, agents
retrieve the body, and the runner executes the blocks — no copies.

## Why

The same file already proved the model on two sides: `.zirkle/memos/routine`
memos are dispatchable procedures, and justdown showed the fenced-just-block
is enough glue for a whole tool surface. A tool whose docs, retrieval
contract and executable are three artifacts drifts; one memo cannot.

## Consequences

Work follows in [[a-tool-is-declared-by-its-memo]]. It sharpens, not replaces, [[every-tool-is-one-command]] (the
command shape) and the event-typed run chain ([[an-event-declares-its-type]],
[[a-listener-subscribes-to-event-types]]): the event type in the frontmatter
is how a watcher tells itself apart from a one-shot. Rust cartridges keep
serving what a memo cannot express; memos carry what they can.
