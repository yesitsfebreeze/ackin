---
kind: note
description: External rating of the zirkle harness against the state of the art, with the improvements it names
date: "2026-09-12"
---

Rating: 8/10 — current-gen architecture, honestly engineered; unproven at the
exact problem the field measured as hardest (long-horizon recall), and a
policy model simpler than the incumbents'.

| Dimension | Score | Against the state of the art |
| --- | --- | --- |
| Compaction | 9 | Matches and hardens the published standard (16 msg / 24 KiB soft trigger, 224 KiB hard budget, 16 KiB excerpts, complete-exchange folding, request kept verbatim). |
| Memory architecture | 8 | Turn ring distilled into the memory bank is the same shape as Mem0's compression engine; pattern current, validation unmeasured. |
| Progressive disclosure | 9 | The resolver makes just-in-time retrieval first-class; beyond what CLI-tool ecosystems ship. |
| Tool surface | 9 | Speaks MCP to external agents instead of inventing a wire. |
| Verification | 8 | Per-cartridge `selftest`/`integration` contracts, `verify` gate, fail-probe; the gate caught a real break this session. |
| Policy / safety | 6 | Rule-based reads-vs-writes only; `ask` with a headless proxy equals deny — see [[headless-policy-approval-channel]]. |
| Long-horizon quality | 5 | No recall benchmark; see [[long-horizon-recall-benchmark]]. |

Session evidence: harness crate 36/36, main workspace 307/307, memory e2e
green; the one gate failure was environmental
([[zirkle-diagnostics-leaks-into-pty-tests]]), not a code regression. The
per-cartridge verification structure extends naturally into
[[per-cartridge-versioned-test-runner]].

Sources: Anthropic, "Effective context engineering for AI agents"
(anthropic.com/engineering/effective-context-engineering-for-ai-agents);
Claude Code best practices (code.claude.com/docs/en/best-practices);
LongMemEval (arXiv 2410.10813); mem0.ai; modelcontextprotocol.io.