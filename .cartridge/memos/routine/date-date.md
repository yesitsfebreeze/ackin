---
kind: routine
description: Work with dates and timestamps using date — format now, convert to and from Unix epoch, and
  compute relative times. Notes the GNU vs BSD/macOS flag split. Use to format a timestamp, get the current
  epoch, convert an epoch to a readable date, or compute a date offset.
uses:
- usage: '[[run-usage]]'
  when:
  - format the current date
  - get the unix timestamp
  - convert epoch to a readable date
  - date arithmetic
  - what was the date 7 days ago
  - iso 8601 timestamp
  - current time in utc
  tags:
  - date
  - time
  - timestamp
  - epoch
  - unix
  - iso8601
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Format and convert timestamps. `now` is an ISO-8601 string; `epoch` the Unix
seconds; `fromepoch` turns seconds back into a readable date; `relative`
computes an offset like "7 days ago". The catch is portability: the **GNU** date
(Linux) and **BSD** date (macOS) differ on the two non-trivial recipes — both
forms are shown.

| Recipe | Does |
|--------|------|
| `now` | current time as ISO-8601 (`-u` for UTC) |
| `epoch` | current Unix timestamp in seconds |
| `fromepoch` | convert a Unix timestamp to a readable date |
| `relative` | a date offset from now (GNU `-d`, BSD `-v`) |

Format codes are shared: `%Y-%m-%d` date, `%H:%M:%S` time, `%s` epoch, `%z`
offset, `%A` weekday. The divergence is only in *parsing/arithmetic*: GNU uses
`-d "7 days ago"`, BSD uses `-v-7d`. The recipes below assume GNU (Linux); the
prose notes the BSD form so a wizard on macOS adapts.

```just
# current time as ISO-8601 (default); pass utc="-u" for UTC
now utc="":
  date {{utc}} +%Y-%m-%dT%H:%M:%S%z

# current Unix timestamp in seconds
epoch:
  date +%s

# convert a Unix timestamp to a readable date (GNU; BSD: date -r SECONDS)
fromepoch seconds:
  date -d "@{{seconds}}" +%Y-%m-%dT%H:%M:%S%z

# a date offset from now (GNU -d; BSD form: date -v-7d)
relative expr="7 days ago":
  date -d {{quote(expr)}} +%Y-%m-%d
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `now` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
