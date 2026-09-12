---
atomic: query-the-record-first
subject: the machine stall this run hits was already on record from pass one, so no time was spent rediscovering it
date: 2026-09-12
updated: 2026-09-12
runs: 4
tags:
  - atomic
---

## Do

1. Run `python3 <pearde>/resources/knowledge.py query "<the contract as a
   question>"` from the board's repo root, before any research outside the
   repo. `<pearde>` is a real absolute path and the brief prints it with an
   `@` alias that does not resolve from a shell — take it from the `pearde`
   executable itself: `dirname $(dirname $(readlink -f $(which pearde)))`, or
   failing that `find / -name knowledge.py -path '*resources*' 2>/dev/null`.
   Never paste the `@` form into a shell; it is a reference, not a path.
2. Read every strong hit at `.pearde/wiki/<type>/<slug>.md` — `query` prints
   the slug and its folder, not the text; a gap enqueues itself and is a
   report line, not a question to the person.

## Done when

- Each strong hit is either used or explicitly set aside, and no outside research was done on a question the record already answered.

## Fails when

- The record is queried after the work rather than before it, so a note that already held the answer is found once it has stopped being worth anything.
