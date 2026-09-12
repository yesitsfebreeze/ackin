---
complexity: 6
footprint:
  - src/loader.rs
---

# spec04 — the document's format is written where its readers are

`src/cartridge.rs` opens with a module header stating the whole process wire —
every frame in both directions — and that header is why the protocol has one
definition rather than one per reader. `src/loader.rs` has no such header, and
the document it parses is now the capability request that [[the-resolver]] and
[[the-sandbox]] will each read. The format is described in `~/dev/sys/CLAUDE.md`,
outside this repo and no longer accurate. This unit moves the definition into
the module that owns it.

## What already stands

Nothing. Every field carries a doc comment on itself, which is the right
information in eleven places and no statement of the document as a whole. There
is no module header on `src/loader.rs`.

## What is left

Write the header: the full shape of `cartridge.json` with every field and
whether it is required, the rule that an absent `grant` is the tightest policy
and not the loosest, the rule that the document wins over the Lua entry
wherever it speaks and the entry is the only source where it does not, and the
subtree rule in one paragraph — private by default, outward only by
re-export, a key unique within a subtree and never across the graph.

Name the two readers and what each takes: the resolver takes `provide`,
`needs` and `export`; the sandbox takes `grant`. Say that there is no second
grant file, because that sentence is the reason a future reader must not add
one.

State what the document does **not** settle, so the next contract is not read
out of this one: nothing here loads a nested cartridge, and where the ledger
looks for cartridges is [[the-ledger]]'s contract, not this format's.

## Acceptance

- [x] `src/loader.rs` opens with a `//!` header, and every field of `Cartridge` and of `Grant` is named in it.
  - Lines 1-60 are the header, carrying the whole document as an annotated JSON
    block with each field marked required or optional.
  - Checked by grepping the header for each field **as a quoted JSON key**, not
    as a substring:
    `for f in name entry binary ui selftest integration source provide needs export grant read write net exec; do printf '%s=%s ' "$f" "$(sed -n '1,60p' src/loader.rs | grep -c "\"$f\"")"; done`
    → `name=1 entry=1 binary=1 ui=1 selftest=1 integration=1 source=1 provide=1
    needs=1 export=1 grant=1 read=1 write=1 net=1 exec=1`. Every field is named
    exactly once, and none of those counts can be reached by accident.
  - The loop in `## Verify and Proof` below counts each field as a **quoted
    JSON key**, which is how the header carries the document's shape; a bare
    substring count could not fail, since `ui` matches "required" and `net`
    matches "nonempty". The replacement was applied by the orchestrator, per
    the report's `### Edits`.
- [x] The header states that an absent `grant` grants nothing.
  - "**An absent `grant` grants nothing.** `Grant::default()` is empty on all
    four of `read`, `write`, `net` and `exec`, which is the tightest policy and
    not the loosest."
- [x] The header states which reader takes which fields, naming the resolver and the sandbox.
  - "**Two readers, one document.** The resolver takes `provide`, `needs` and
    `export` and binds them; the sandbox takes `grant` and confines to it.
    There is no second grant file and no separate policy document."
- [x] The header states that nothing in this module loads a nested cartridge.
  - "**What this module does not settle.** Nothing here loads a nested
    cartridge: `Cartridge::offered` reads a subtree off the filesystem to
    validate a re-export, and that is all. Where the ledger looks for
    cartridges, and how a need is resolved outward through subtrees, are the
    ledger's and the resolver's contracts, not this format's."
- [x] No field's own doc comment is deleted to make room — the header states the document, the field comments state the field.
  - `git diff src/loader.rs | grep '^-.*///'` returns **two** lines, and both
    are `Cartridge::read`'s own method header:
    ```
    -	/// Read a manifest and resolve the entry it names. The loader and the
    -	/// bundler share this, so what ships and what loads agree on the format.
    ```
    They were replaced by a longer header when `read` was split from
    `document`, not removed. **No field's doc comment is touched**: all
    thirteen on `Cartridge` and `Grant` are present — nine on `Cartridge`
    (`binary`, `ui`, `selftest`, `integration`, `source`, `provide`, `needs`,
    `export`, `grant`) and four on `Grant`. `name` and `entry` carry none and
    never did; they are the two required fields and the header names both.
  - The file holds 119 `///` lines in all, 82 of them indented inside a struct
    or an impl. Last pass this box quoted "68 remain", which counted only one
    indent form and was wrong.
- [x] `cargo doc --offline --no-deps` builds with `warnings = "deny"` in force.
  - `Finished \`dev\` profile ... in 0.34s` /
    `Generated .../target/doc/zirkle/index.html`, exit 0.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-manifest
sed -n '1,60p' src/loader.rs
for f in name entry binary ui selftest integration source provide needs export grant read write net exec; do
	printf '%s=%s ' "$f" "$(sed -n '1,60p' src/loader.rs | grep -c "\"$f\"")"
done; echo
cargo doc --offline --no-deps
```
