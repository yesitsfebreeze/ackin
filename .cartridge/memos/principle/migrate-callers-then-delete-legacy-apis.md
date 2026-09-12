---
kind: principle
description: Apply when introducing a new internal API while old callers still exist. Migrate callers and delete the old API in the same wave instead of preserving compatibility layers.
group: architecture
---

# Migrate callers then delete legacy APIs

When a new API is the right design, migrate callers and remove the old API in
the same refactor wave instead of preserving compatibility layers.

**The rule:**

- Do not keep a legacy path only because internal callers still exist.
- Inventory the callers, migrate them, delete the old API immediately.
- Treat temporary adapters as exceptional and time-boxed, not as default
  architecture.
- Update tests to assert the new contract, and delete tests that only protect
  pre-refactor implementation details.

**When this applies:** no external users depend on backward compatibility, the
project can absorb coordinated breaking changes, and the new API is part of a
simplification.

Keeping both old and new creates dual-path complexity, slows cleanup, and
makes the codebase feel append-only. This is the API-shaped case of
[[outcome-oriented-execution]], and the same law this record states as
[[delete-superseded-work]].
