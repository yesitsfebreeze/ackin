---
kind: routine
description: Open an agent review of the current PR. Use when the user asks to review, critique, or sanity-check
  a pull request.
uses:
- usage: '[[run-usage]]'
  when:
  - Open an agent review of the current PR. Use when the user asks to review, critique, or sanity-check
    a pull request.
  tags:
  - review
  - pr
  - agent
  - github
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

**Brief.** Summarize the diff, flag risks, and propose a verdict
(approve / request changes / block). Be concise; prefer a short bulleted risk
list over prose. Do not nitpick style — that is [[tools-fmt]]#check's job.

```just
# fetch the PR diff and hand it to a reviewer model
review pr := `gh pr view --json number,title,body -q .number`:
  diff=`gh pr diff {{pr}}`
  claude --print --prompt "Review this PR diff. $(diff)"
```

## Notes

- `pr` defaults to the PR on the current branch; pass a number to review a different PR.

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `review` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
