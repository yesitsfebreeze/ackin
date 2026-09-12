---
kind: routine
description: Adversarially review a change from several independent angles, synthesize the findings, and return one lead verdict bucketed into act on, consider, noted and dismissed.
uses:
  - usage: "[[run-usage]]"
    when: [stress-testing a contested design before shipping, hunting blind spots in a changeset, asking for an adversarial review]
---

Dispatch one reviewer per configured model to adversarially review a change.
Every reviewer gets the same prompt and rubric — the adversarial signal comes
from model diversity, not from assigned personas.

The deliverable is a synthesized verdict. Do not auto-apply changes.

1. Determine scope

- The caller pointed at files or a diff: use that.
- On a feature branch: `git diff main...HEAD` for the full changeset.
- The caller referenced recent work: gather the relevant files.

Package the diff plus the surrounding context files reviewers need to
understand the code.

2. State the intent

Before dispatching, write one clear paragraph on what the change is trying to
do. Derive it from the caller's message, the commit messages, the PR
description and the code itself. A review against an unstated intent produces
findings about the wrong thing. If the intent is genuinely unclear, ask before
proceeding — this is the narrow case [[never-block-on-the-human]] leaves open,
because proceeding on the wrong intent wastes every reviewer.

3. Dispatch reviewers

Launch all reviewers in one message, read-only, one per configured model.
Prefer different model families. Each gets the same filled template: the
stated intent, the diff or file contents, the review rubric, and the
code-quality lens.

If a model is unavailable, pick the closest equivalent in the same family,
proceed, and note the substitution. Do not block the review on it.

4. Synthesize

- Parse all findings.
- **Consensus.** Findings raised by two or more reviewers independently are
  the highest signal.
- **Lone findings.** Still worth reading, weighted accordingly.
- **Deduplicate.** Different models describe the same issue differently. Merge
  them and note who raised it.
- **Disagreements.** One model flagging what another explicitly cleared is
  useful context for the verdict.

5. Lead judgment

You are the lead reviewer, a pragmatic senior engineer, not a neutral
aggregator. Categorize every finding:

- **Act on.** Real issues in correctness, security or maintainability given
  the actual goals. These would block a real change.
- **Consider.** Legitimate, but the cost of addressing them now may outweigh
  the benefit. Worth the caller's attention.
- **Noted.** Technically valid, not actionable. Context-dependent, premature,
  or low impact at this stage.
- **Dismissed.** Wrong, nitpicky, or missing context. Say briefly why.

Each finding names which reviewers raised it, its bucket, and a one-line
rationale for the bucket.

6. Present

Intent, reviewers with finding counts, then Act on, Consider, Noted,
Dismissed, then an agreement map saying where models agreed, where they
diverged, and what the pattern means.

Check: every finding lands in exactly one bucket with a rationale. An
unbucketed finding list is an aggregation, not a verdict.

Failure: reviewers agree unanimously and shallowly. Unanimous praise on a
non-trivial change means the intent was stated too narrowly or the diff lacked
context. Re-scope and rerun rather than shipping on it.
