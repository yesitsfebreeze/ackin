---
kind: note
description: "cordiverse/cordis on GitHub, read at commit f8ea3cd (v4.0.0-rc.10, 2026-09-08); TypeScript, API declared unstable"
uses:
  - usage: "[[read-usage]]"
    when: ["reading Cordis source or checking what the paper's algorithms became in code"]
    tags: [reference]
---

# cordis-repo

<https://github.com/cordiverse/cordis>, pinned at
[f8ea3cd50f1a5724e8e715995bcde131c9c12b2c](https://github.com/cordiverse/cordis/commit/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c),
package `cordis@4.0.0-rc.10`. The README says the API may change without
notice. Primer (incomplete, marked under construction):
<https://deepseek-harness.github.io/deepseek-harness/reference/cordis-primer>.
Cordis is the plugin kernel of DeepSeek Harness; Koishi, the paper's case
study, still runs Cordis v3.

Files that carry the model, all under `packages/`:

- `core/src/fiber.ts` — lifecycle, effects, epochs (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/fiber.ts#L229-L459)
- `core/src/reflect.ts` — provide, set, dependent notification, proxy access (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/reflect.ts#L61-L228)
- `core/src/context.ts` — root, extend, isolate, intercept (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/context.ts#L36-L76)
- `core/src/events.ts` — emit, parallel, serial, bail, waterfall (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/events.ts#L72-L174)
- `loader/src/config/` — entries, groups, trees; `include/src/` — file-backed config; `hmr/src/index.ts` — hot replacement

Read through the GitHub API only; nothing cloned. Paper: [[cordis-paper]].
