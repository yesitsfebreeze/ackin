---
kind: type
type: system
description: One component of the system prompt
uses:
  - usage: "[[read-usage]]"
    when: [understanding system prompt composition]
  - usage: "[[compose-usage]]"
    when: [writing a system memo, understanding system prompt composition]
---

# System prompt memos

A `kind: system` memo contributes its Markdown body to the system prompt.
Use one memo per coherent instruction or context section. `description`
explains its purpose. Optional integer `order` defaults to zero; lower values
come first, with record-relative path as the stable tie-breaker. Optional
boolean `enabled` defaults to true; false keeps a memo out of the prompt.
All enabled system memos in the nearest `.cartridge/memos/` are composed. No other
kind is included automatically. Link to supporting memos for discovery.

The harness renders exact single-backtick placeholders after composition:
`system` is optional profile base text, `cwd` is the session directory, `date`
is the current date, `environment` is the wrapped shell's host scan (shell,
OS, user, terminal, PATH and its executables; blank in profiles without
one), `terminal` is the wrapped shell's recent commands with exit codes and
output (blank without one or before its first prompt), `anchor` is the
session, shell foreground and touched files this request was made in (blank
without a wrapped shell), `instructions` contains labelled workspace guidance, and
`summary` contains labelled compacted conversation context. Unknown names,
fenced code, and multiple-backtick spans remain literal. Inserted values are
not interpreted again. Include instructions and summary placeholders when
replacing the default context memos, so guidance and conversation context stay
available.
