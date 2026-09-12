---
kind: principle
description: Apply when context is filling up — large outputs, long files, repeated reads, fan-out planning. Route bulk to subagents and keep summaries in the main thread, not raw payloads.
group: delegation
---

# Guard the context window

The context window is finite and non-renewable within a session. Every token
should be worth its cost.

**Why:** Overflow degrades reasoning quality, creates compression artifacts,
and halts progress.

**The pattern:**

- **Isolate large payloads.** Route verbose outputs, screenshots and large
  documents to subagents. The main context gets summaries, not raw data.
- **Do not read what you will not use.** Read selectively. A file not needed
  for the current task is a file to skip.
- **Keep frequently used content inline.** A reference used on every
  invocation belongs in the routine itself, not in a separate file that costs
  a read each time.
- **Size phases and cap scope.** Limit files per phase, set turn budgets,
  account for the cost of the mechanism itself.

The human analogue is [[minimize-reader-load]] — working memory is finite for
readers too.
