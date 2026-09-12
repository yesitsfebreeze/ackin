---
kind: routine
description: Query DNS with dig — resolve a name, get just the answer, look up a record type, reverse
  an IP, or trace delegation from the root. Use to debug DNS, check a record, or see what an authoritative
  server returns.
uses:
- usage: '[[run-usage]]'
  when:
  - resolve a hostname
  - look up a DNS record
  - check MX or TXT record
  - reverse DNS lookup
  - debug DNS
  - query a specific nameserver
  - trace DNS delegation
  tags:
  - net
  - dig
  - dns
  - resolve
  - records
  - nameserver
---

## Inputs

Requires on PATH: `dig`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

The DNS debugger. `short` gives just the answer for quick checks; `lookup`
queries any record type; the rest reverse an IP, trace delegation, or query a
named server directly. Pairs with [[net-ss]] and [[proc-lsof]] when a connection
fails and you need to tell *name resolution* problems from *socket* problems.

| Recipe | Form | Returns |
|--------|------|---------|
| `short` | `+short` | just the answer value(s), nothing else |
| `lookup` | `<name> <type>` | full answer section for a record type |
| `reverse` | `-x ip` | the PTR (name) for an IP address |
| `trace` | `+trace` | the full delegation path from the root servers |
| `at` | `@server` | query a specific resolver, bypassing the default |

Record types for `lookup`: `A`/`AAAA` (IPv4/6), `MX` (mail), `TXT` (SPF, verify),
`NS` (nameservers), `CNAME` (alias), `SOA` (zone authority). `+short` composes
with any query. `@1.1.1.1` or `@8.8.8.8` checks what a public resolver sees vs
your local one — the fastest way to spot a stale or split-horizon record.

```just
# just the answer for a name (A record by default)
short name:
  dig +short {{quote(name)}}

# full answer for a specific record type (A, AAAA, MX, TXT, NS, CNAME, SOA)
lookup name type="A":
  dig {{quote(name)}} {{quote(type)}} +noall +answer

# reverse lookup: the PTR name for an IP address
reverse ip:
  dig -x {{quote(ip)}} +short

# trace the delegation path from the root servers down
trace name:
  dig {{quote(name)}} +trace

# query a specific resolver (e.g. 1.1.1.1) instead of the default
at server name type="A":
  dig @{{server}} {{quote(name)}} {{quote(type)}} +noall +answer
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `short` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
