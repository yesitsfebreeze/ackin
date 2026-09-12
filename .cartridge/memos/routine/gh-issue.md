---
kind: routine
description: Manage GitHub issues — create, list, view, comment, close, edit. Use to file an issue, triage
  or inspect the queue, update labels/assignees, comment, or close.
uses:
- usage: '[[run-usage]]'
  when:
  - file an issue
  - create an issue
  - list issues
  - view an issue
  - comment on an issue
  - close an issue
  - edit an issue
  - triage
  tags:
  - gh
  - github
  - issue
  - triage
---

## Inputs

Requires on PATH: `gh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

Issue control surface, sibling of [[gh-pr]]. `list` is the default; the rest are
callable by name. Repo targeting and JSON output come from [[gh-cli]]. All writes
hit the remote (`side_effects: network`).

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `create` | `gh issue create` | title + body |
| `list` | `gh issue list` | filter by `state` (open/closed/all) |
| `view` | `gh issue view` | `number` = number/URL |
| `comment` | `gh issue comment` | one comment |
| `close` | `gh issue close` | resolve an issue |
| `edit` | `gh issue edit` | add labels and assignees |

```just
# file an issue
create title body:
  gh issue create --title {{quote(title)}} --body {{quote(body)}}

# list issues by state (default open)
list state="open":
  gh issue list --state {{quote(state)}}

# view an issue (number or URL)
view number:
  gh issue view {{quote(number)}}

# comment on an issue
comment number body:
  gh issue comment {{quote(number)}} --body {{quote(body)}}

# close an issue
close number:
  gh issue close {{quote(number)}}

# edit an issue — add a label and/or assignee (blank = skip)
edit number label="" assignee="":
  #!/usr/bin/env sh
  set -eu
  n={{quote(number)}}; l={{quote(label)}}; a={{quote(assignee)}}
  set -- "$n"
  [ -n "$l" ] && set -- "$@" --add-label "$l"
  [ -n "$a" ] && set -- "$@" --add-assignee "$a"
  gh issue edit "$@"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
