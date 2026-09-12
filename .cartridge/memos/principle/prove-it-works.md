---
kind: principle
description: Apply after completing a task, before declaring done. Verify against the real artifact — run the feature, read the actual value, inspect the diff — not a proxy, a self-report, or "it compiles".
group: verification
---

# Prove it works

Verify every output by checking the real thing directly. Do not infer from
proxies, self-reports, or "it compiles".

**Why:** Unverified work has unknown correctness. Indirect verification —
file timestamps, output freshness, an agent's own summary, a cached
screenshot — feels cheaper than direct observation. Acting on a wrong
inference costs far more than checking the source.

**The pattern.** After completing any task, ask: how do I prove this actually
works?

Check the real thing, not a proxy:

- Check process liveness directly, not through derived state.
- Read the actual value, not a cached or derived representation.
- When verification fails, suspect the observation method before suspecting
  the system.

For code and features:

1. Build it. Necessary, not sufficient.
2. Run it and exercise the actual feature path.
3. Check the full chain — does data flow from input to output?
4. For integrations, test the full communication path end to end.

For delegated work, trust artifacts, not self-reports. Inspect the diff, the
file contents, the runtime behaviour — never the delegate's summary.

**Script the check when you can.** The strongest proof is a deterministic
script that reruns the same comparison, not a one-time eyeball. Write it, run
it, and keep its output as an artifact a reviewer can rerun instead of
trusting your word. That is [[build-the-lever]] applied to verification.

Keep the artifact visible. Commit it only for work large enough that the trail
has to be auditable later — see [[show-me-your-work]].

The sequencing complement is [[sequence-verifiable-units]], which says when to
run each check.
