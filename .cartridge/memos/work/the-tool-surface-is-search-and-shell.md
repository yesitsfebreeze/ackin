---
kind: work
description: "The enabled tool surface is one search plus the user's shell: the file tools and the query ops the landscape already answers go"
status: open
level: 10
estimate: 3h
needs:
  - "[[one-search-covers-the-record-and-memory]]"
uses:
  - usage: "[[read-usage]]"
    when: ["deciding whether a tool belongs on the surface, or trimming what the agent is handed"]
---

# the-tool-surface-is-search-and-shell

## Outcome

The tools a profile hands an agent are the ones it cannot get another way: one
search over what the system remembers, the user's own shell, and the writes that
must be validated (the record, memory, git). Everything that exists only to read
or list something the search already answers is gone, so the surface stops being
a catalogue to choose from.

What goes: `builtin/fs` — six tool keys (`read`, `write`, `edit`, `search`,
`glob`, `grep`) that no shipped profile composes — and the memo ops that restate
the landscape (`index`, whose kind histogram the graph carries as node counts,
and `coverage`, whose undeclared-file report is a query over the same rows).

Scope is the enabled surface and the memo op set. The tool envelope itself, and
serving these tools outside zirkle, are [[every-tool-is-one-command]] and
[[zirkles-tools-serve-any-agent]].

## Check

- [ ] No cartridge in `builtin/` provides tool keys that no profile in
      `.zirkle/` composes: `builtin/fs` is deleted, its workspace member and
      `.zirkle/memos` record with it.
- [ ] The glob and dispatch-removal fixtures that used `fs` prove the same
      behaviour with a composed cartridge instead: `cargo test -p zirkle --lib
      tests::profile` passes with no `with_filesystem_tools` helper in the tree.
- [ ] `tool.memo`'s op enum no longer lists `index` or `coverage`, and the
      landscape query answers both questions: a kind overview from its node
      counts, and undeclared files from a query whose hits name them.
- [ ] The agent's dispatch list in the default profile is exactly the keys its
      composed cartridges provide, with nothing on it that reads a file the shell
      can read.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12, auditing whether each cartridge is only what it needs to
be. `builtin/fs` is 116K and six tool keys; no profile in `.zirkle/` lists it,
and the only references are `with_filesystem_tools` in `core/tests/profile.rs`,
which injects it into a fixture copy of the default profile to prove the glob
expansion and dispatch removal. So it ships nothing and is kept alive by the
tests that were written against it — those fixtures should assert the same
properties with a cartridge the product actually composes.

The premise, from the user: with a shell and one search, reading files is `cat`,
and what matters instead is knowing the shell environment reliably — which `pty`
already provides as the `environment` scan and the `terminal` command history,
both rendered into the prompt. A file-reading tool adds a second way to do what
the shell does, and every added key is one more thing the model has to choose
between.

Order: this needs [[one-search-covers-the-record-and-memory]] first, because
`index` and `coverage` may only go once the query that replaces them answers
across both stores. Deleting `builtin/fs` needs nothing and can land on its own.
