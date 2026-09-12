---
kind: routine
description: Fetch a URL with curl — read the body, dump only the response headers, do a HEAD probe, or
  save the resource to disk. Use to read or download an HTTP resource without mutating the remote.
uses:
- usage: '[[run-usage]]'
  when:
  - fetch a url
  - http get
  - download a file
  - read response headers
  - http head request
  - save a url to disk
  - check content-type
  tags:
  - http
  - get
  - curl
  - download
  - headers
---

## Inputs

Requires on PATH: `curl`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger none.

## Do

Read-only curl fetches. `get` is the default — follows redirects (`-L`) and fails
loud on 4xx/5xx (`-fsS`) so it is safe to script. `head` and `headers` inspect
without pulling the body; `download` writes to disk. Flag meanings live in
[[http-curl]]; status families in [[http-status]]. Pipe a JSON body into [[search-jq]] to
slice it.

| Recipe | curl | Returns |
|--------|------|---------|
| `get` | `-fsSL` | the body, redirects followed |
| `head` | `-fsSIL` | response headers only (HEAD request) |
| `headers` | `-fsSL -D - -o /dev/null` | headers from a real GET, body discarded |
| `download` | `-fsSL -O` (or `-o out`) | resource saved to disk |

`head` issues a HEAD; some servers answer it differently than a GET, so use
`headers` when you need the exact headers a GET would return. `download` with no
`out` keeps the URL's filename; pass `out` to rename.

```just
# fetch the body, following redirects (default)
get url:
  curl -fsSL {{quote(url)}}

# HEAD request — status line and headers, no body
head url:
  curl -fsSIL {{quote(url)}}

# dump the headers of a real GET, discard the body
headers url:
  curl -fsSL -D - -o /dev/null {{quote(url)}}

# save the resource to disk; out="" keeps the remote filename
download url out="":
  #!/usr/bin/env sh
  set -eu
  u={{quote(url)}}
  o={{quote(out)}}
  if [ -n "$o" ]; then
    curl -fsSL -o "$o" "$u"
  else
    curl -fsSL -O "$u"
  fi
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `get` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
