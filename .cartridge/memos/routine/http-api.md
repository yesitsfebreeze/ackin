---
kind: routine
description: Thin REST helper over curl — call any method on a URL with a bearer token and JSON body,
  or upload a multipart form. Use to hit an authenticated JSON API or post a file as form data.
uses:
- usage: '[[run-usage]]'
  when:
  - call a rest api
  - authenticated http request
  - bearer token request
  - post json to an api
  - upload a file
  - multipart form upload
  tags:
  - http
  - api
  - rest
  - auth
  - curl
---

## Inputs

Requires on PATH: `curl`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

curl wired for JSON APIs. `call` takes a method, URL, and JSON body and always
sends `Content-Type: application/json`. `auth` adds an `Authorization: Bearer`
header for protected endpoints; `form` does a `-F` multipart upload (files or
fields), letting curl pick the boundary and Content-Type. All run `-fsS`, so a
4xx/5xx exits non-zero — see [[http-status]]. For the bare verbs without JSON
defaults use [[http-send]]; for the flags see [[http-curl]]. Pipe the response straight
into [[search-jq]] to pull out fields.

| Recipe | Adds | Body |
|--------|------|------|
| `call` | `Content-Type: application/json` | JSON via `-d` |
| `auth` | the above + `Authorization: Bearer <token>` | JSON via `-d` |
| `form` | multipart, boundary auto | `-F key=value` (`-F file=@path` to upload) |

`form` fields follow curl's `-F` syntax: `name=value` for a plain field,
`name=@path` to attach a file. The token in `auth` is the raw token; the
`Bearer ` prefix is added for you.

```just
# call a JSON API: method + url + JSON body (default)
call method url body:
  curl -fsS -X {{quote(method)}} -H 'Content-Type: application/json' -d {{quote(body)}} {{quote(url)}}

# same, with a bearer token for protected endpoints
auth method url body token:
  #!/usr/bin/env sh
  set -eu
  m={{quote(method)}}
  u={{quote(url)}}
  b={{quote(body)}}
  t={{quote(token)}}
  curl -fsS -X "$m" -H 'Content-Type: application/json' -H "Authorization: Bearer $t" -d "$b" "$u"

# multipart upload: each field is key=value or file=@path
form url +fields:
  curl -fsS -X POST -F {{fields}} {{quote(url)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `call` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
