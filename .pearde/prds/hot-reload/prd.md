---
state: done
origin: requested
priority: 65
complexity: 13
blast-radius: mid
needs:
  - the-resolver
  - the-wire
actual: 0.01h
---


# Hot reload

Replace one node while the tree is running. What stood on it re-resolves against
the replacement; what stood beside it is untouched.

The transaction is already implemented and already proven — `~/dev/sys/core/README.md`
describes it and `src/runtime.rs` and `src/fiber.rs` carry it: prepare the old
cartridge, drain its service calls, freeze its bank, hand the candidate a private
copy, publish only an active candidate presenting the same provided keys,
unfreeze the old bank when the candidate is rejected, and dispose in dependency
order. That logic survives this PRD intact.

What changes is scope. Reload was a profile operation; here it is a subtree
operation. The blast radius of replacing a node is exactly its dependents, and
[[the-resolver]] makes that set knowable from the process tree without any
bookkeeping — the tree already is the answer.

Must not change: `host.on_reload` receiving `true` to prepare and `false` to
cancel, and the rule that a candidate needing an exclusive resource still held by
the old process must fail preparation rather than bind it twice.

At the end, a cartridge is rebuilt and swapped under a running tree, its
dependents follow it, and nothing else notices.

## History

**failed, retried 2026-09-12 20:35**

spec01: exit 2

running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 111 filtered out; finished in 0.07s

bash: line 3: test: too many arguments

## Report

spec01: exit 0

running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 111 filtered out; finished in 0.07s

spec02: exit 0

running 14 tests
..............
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 98 filtered out; finished in 4.31s

{"msg":"runtime error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: migration rejected\nstack traceback:\n\t[C]: in function 'error'\n\t[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: in function <[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1>","src":"p","t":1789238133356,"turn":null}
{"msg":"runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/.tmpPy1sfx/p/cartridge.json: expected value at line 1 column 1","src":"instance","t":1789238133357,"turn":null}
{"msg":"no running node carries uid 2","src":"2","t":1789238133402,"turn":null}
{"msg":"rejected fixture migration","src":"peer","t":1789238133853,"turn":null}
{"msg":"child refused reload","src":"parent","t":1789238134341,"turn":null}
