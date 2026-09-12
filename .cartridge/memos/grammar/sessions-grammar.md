---
kind: grammar
description: "the sessions cartridge and its dir, agent chat sessions, and the profile entry's retained state"
overloads: the sessions cartridge, chat sessions, the sessions.dir config
date: "2026-09-12"
---

# sessions

Three senses. The **sessions** cartridge (`builtin/sessions`) owns buffers,
files touched and agent state, persisted per session under the required
`sessions.dir` configuration — that config key is sometimes called "sessions"
shorthand. A **chat session** is the conversation container in the UI agent
backend: `send` starts one, runs attach to it, each dispatch is fresh. And
profile entries live across **replacements** — a chat session and the
sessions cartridge's per-session state are different stores with different
lifetimes. Authority is `builtin/sessions/README.md` and [[shell-grammar]]
for what a session wraps.

The bite: "session" alone does not say which store. The cartridge persists
agent state; the chat backend owns conversation runs; neither is the other.
See [[memo-grammar]] for where session state may be recorded instead.