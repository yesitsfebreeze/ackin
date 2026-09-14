---
kind: note
description: Evidence and scope behind the September 2026 cartridge improvement work programme
date: "2026-09-13"
uses:
  - usage: "[[read-usage]]"
    when: [understanding the cartridge improvement assessment, checking the evidence behind cartridge work plans]
---

# Evidence for cartridge improvement plans

The user requested implementation-ready plans for all 15 cartridge assessments,
with 2–3 improvements and three downsides each. The programme covers the 45
improvements and maps the 45 downsides to mitigations or retained limitations.
Scores in the conversation were subjective usefulness judgments, not measurements.

## Observed in this conversation

On 2026-09-13, memo fabric/resolve/types and PTY readback succeeded. GitFS
`ls` returned an approval-required tool error because the MCP profile has no
interactive approval channel for that operation. Memory query returned a competing
writer error. The PTY reported Nushell with cwd in cartridge.ctg; Codex's command
environment reported Zsh in the parent checkout. These are a point-in-time baseline,
not permanent facts about every profile.

The live composition exposed gitfs/ship, memo, memory and shell tools. Agent, fs,
harness, proxy, router, tools and UI were disabled in that MCP profile. Registration
as Active did not establish that the memory store was usable.

## Source findings

- gitfs.ctg/service.rs snapshot dispatch enumerates all owned paths without applying
  the advertised input.paths filter; prove this in a regression fixture before fixing.
- gitfs.ctg/service.rs and ship.rs return object-valued success content; mcp.ctg/service.rs
  and proxy.ctg/service.rs require string-valued content. The wire disagreement is
  visible in source; the exact real-client failure is a reproduction obligation.
- gitfs.ctg/ship.rs has hardcoded Claude Code coauthor attribution and an optional
  gate whose failure behavior must be made explicit in any shipping preview.
- policy.ctg/init.lua has tool-wide configurable rules and special cases for memo,
  memory and shell readback; read-only GitFS has no equivalent special case.
- memory-tool.ctg/src/main.rs exposes query/ingest, accepts an undocumented sync
  execution field, and does not enforce all schema bounds at dispatch.
- harness already includes compaction fixtures and an inspector; PTY already has
  command marks/IDs; memo already has index/read and revision metadata; fs already
  has freshness tests. Plans audit and extend these mechanisms instead of assuming
  they are missing.

Paths above are relative to /Users/feb/dev/cartridge. Current source and manifests
take precedence over historical builtin/, core/, .zirkle/ or zirkle command references.

## Other evidence and overlap

The existing /Users/feb/dev/cartridge/.cartridge/CARTRIDGE-ASSESSMENT.md reports a real temporary
MCP GitFS/ship result-envelope failure and passing component suites. This planning
pass did not rerun those suites and does not claim their results as fresh evidence.
That report also proposes fabric centralization, document execution and repository
consolidation. Those architectural migrations are context for avoiding duplicate
work, not assumed user-approved prerequisites for the 45 improvements here.

Reuse the scope and evidence in [[headless-policy-approval-channel]],
[[rolling-context-retains-decision-evidence]], [[long-horizon-recall-benchmark]],
[[a-search-ranks-current-guidance-over-delivered-history]],
[[every-enabled-tool-ships-a-contract-probe]] and
[[one-search-covers-the-record-and-memory]]. In particular the headless record
already chose explicit profile grants; this programme extends their granularity,
rather than claiming that a client necessarily prompts before every MCP call.

Existing active claims include agents-query-the-tool-graph, pty-encodes-input,
the-sidebar-slides-over-the-shell, the-board-is-channels-of-lines and
rpc-contracts-run-across-rust-lua-and-bun. Check their live ownership before touching
overlapping files; no owner or status was changed by this planning pass.

## Scope and evidence discipline

Implementation work is open and unclaimed. Planning does not launch daemons, modify
live policy, ingest private memory, ship commits, or alter stores. Memory implementation
must follow memory.ctg/AGENTS.md with an isolated branch/worktree. Future acceptance
checks use temporary repositories/stores and local provider fixtures by default.
Live model evaluation is a separately selected execution step with explicit configuration
and a bounded budget. A failed dependency or uncertain mutation is never recorded as
an empty successful result.
