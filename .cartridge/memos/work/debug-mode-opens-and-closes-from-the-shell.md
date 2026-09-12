---
kind: work
description: "The agent enters, queries and leaves debug mode from the wrapped shell against the running host"
status: blocked
owner: "sys-opus-2026-09-12/implementer-debug-mode-opens-and-closes-from-the-shell"
level: 11
priority: P1
estimate: 1d
---

# Debug mode opens and closes from the shell

**Eight of nine boxes are closed and observed on the renamed tree, and
`just check` is green (exit 0). The ninth needs `just test`, which fails on one
memo the memory cartridge ships:
`builtin/memory/.zirkle/memos/research/the-watcher-stores-an-empty-body-as-a-document.md`
has an unquoted `text: ""` inside its `description:`, so its frontmatter is
invalid YAML and `test_router.py`'s `LaunchedAgent` cannot boot the cartridge.
That file is in the gitignored, independently versioned memory workspace every
lane shares; the trunk fails the same tests harder. Quote that description and
the last box is one `just test`.**

## Outcome

The agent turns diagnosis on and off itself, with no manual UI toggle: entering
debug mode, asking its status and leaving it are shell commands it can discover
through `tool.memo` and run through `tool.shell`. Leaving restores the normal
diagnostic settings and leaves the user's terminal session usable — the shell,
its cwd and its running program survive the round trip.

The commands address the **running** host, not a second copy of the profile.
The analyst probe on `work/debug-mode-correlates-a-terminal-turn` landed that
half on branch `work/debug-mode-correlates-a-terminal-turn` (commit `165cd6d`):
`zirkle run` now serves the profile socket for as long as the foreground call
lives, so `zirkle status` and `zirkle call` from the wrapped shell reach the live
process. `core/tests/test_run_socket.py` proves it and was seen failing without
the change. Continue that lane.

What is still missing: a debug surface to call. `core/main.rs`'s `Command` enum
stops at `Run/Launch/Daemon/Send/Tail/Call/Reload/Status/Socket/List`, and no
cartridge provides a `debug` key. `zirkle status` already reports every fiber with
its state, injections, provides and error, so status is mostly a rendering of
what exists rather than new bookkeeping.

Two constraints the probe hit and this memo must settle, because a debug entry
point calls services the same way an agent tool does:

- `zirkle call tool.shell …` over the socket answers `invocation context
  required`.
- `zirkle call memo …` over the socket answers `trusted memo cwd required`.

Scope: the lifecycle only. Enumerating what can be probed is
[[every-enabled-tool-ships-a-contract-probe]]; running the probes is
[[tool-probes-run-and-locate-a-failing-provider]].

## Check

- [x] `zirkle debug` against a running host prints `"on":false`, `zirkle debug on`
      prints `"on":true` with the log path, and `zirkle debug off` prints
      `"on":false` again — each exiting 0.
- [x] Debug mode answers while the host is busy inside a service call: the
      foreground-run fixture holds the Lua interpreter for the whole test and
      the three commands still answer.
- [x] The host survives the round trip: after `on` and `off` the `zirkle run`
      process is still alive and `zirkle status` lists the same fibers in the
      same states as before.
- [x] While on, a host event reaches `.zirkle/logs/debug.log` as one JSON object
      per line; after `off` the file stops growing; entering twice does not
      double the lines; the `bytes` `zirkle debug` reports equals the file size.
- [x] `zirkle debug off` without a preceding `on` exits 0 and reports `"on":false`.
- [x] A `zirkle debug`/`status`/`call` whose host goes away mid-request exits
      non-zero with a message instead of exiting 0 with empty output.
- [x] `zirkle call memo '{"op":"list","kind":"routine","limit":500,"cwd":"<repo>"}'`
      against a running default-profile host lists `zirkle-debug`, so the agent
      discovers the commands through `tool.memo`.
- [x] `zirkle call tool.shell '{"op":"call","input":{"command":"echo probe-ok"},
      "context":{…}}'` against that host runs in the user's own shell and
      returns `probe-ok`, while the same call without `context` answers
      `invocation context required` — the two refusals this memo names are
      missing arguments, not socket limits.
- [ ] `just check` and `just test` are green.

```sh
cd /Users/feb/dev/sys
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# The fixtures: lifecycle against a live foreground run, tap content and
# argument validation against a daemon. Both are hermetic temporary profiles.
python3 -m unittest discover -s core/tests -p test_debug_mode.py
python3 -m unittest discover -s core/tests -p test_run_socket.py

# End to end on the real default profile. Cartridge executables are found
# beside the running binary, so this must be `target/debug/zirkle`, never a copy.
M=target/debug/zirkle
$M debug >/dev/null 2>&1 || { $M daemon >/tmp/zirkle-debug-check.log 2>&1 &
  for i in $(seq 60); do $M debug >/dev/null 2>&1 && break; sleep 1; done; }
$M debug
$M debug on
$M call tool.shell '{"op":"call","input":{"command":"echo probe-ok"}}'
$M call tool.shell '{"op":"call","input":{"command":"echo probe-ok"},"context":{"session":"check","turn":1}}'
$M call memo "{\"op\":\"list\",\"kind\":\"routine\",\"limit\":500,\"cwd\":\"$PWD\"}" | grep -c zirkle-debug
$M debug
wc -l .zirkle/logs/debug.log
$M debug off
pkill -f 'target/debug/zirkle daemon'

just check
just test
```

## Approach

The probe is committed on lane `work/debug-mode-opens-and-closes-from-the-shell`
as `2a225bc` (the inherited socket commit, cherry-picked from
`work/debug-mode-correlates-a-terminal-turn`) and `c252a88` (this memo's work).
It builds and passes; what is left is the two defects it exposed and the gates.

### What debug mode is

One diagnostic setting, held by the host, not a cartridge: **while debug mode is
on, every message the host puts on its socket — cartridge events, cartridge
errors and fiber transitions — is also appended to `.zirkle/logs/debug.log`, one
JSON object per line.** Leaving drops the tap, which is the whole setting, so
the host is back to normal. That is the evidence channel the correlated-turn
sibling ([[a-turn-carries-one-id-through-host-lua-and-bun]]) will stamp ids
into, and the one the probe runner
([[tool-probes-run-and-locate-a-failing-provider]]) reads after a probe.

A file rather than `zirkle tail` because an agent must not hold a stream open in
the user's shell: turn on, reproduce, read the file, turn off.

### What the probe built

- `core/lua.rs` — `Host::debug_log()` and `Host::debug(Option<bool>)`, plus one
  `debug: Mutex<Option<JoinHandle>>` field. The tap is a spawned task selecting
  over the same two streams the socket writer merges (`outbox` and
  `runtime().lifecycle()`) and appending each as a line. `Some(true)` when
  already on is a no-op, so entering twice keeps one tap; `Some(false)` aborts
  and drops it; `None` only reports. Every answer is
  `{"on":…,"log":…,"bytes":…}`.
- `core/socket.rs` — `debug: Option<String>` on `Request`, answered beside
  `status` with `{"debug": …}`. It needs no Lua and no fiber, which is why it
  still answers while a service call holds the interpreter.
- `core/main.rs` — `Command::Debug { state }`, `value_parser = ["on","off",
  "status"]`, defaulting to `status`, sent through the existing `ask` helper.
- `.zirkle/memos/routine/zirkle-debug.md` — how the agent finds the commands through
  `tool.memo`. Verified: `op:list, kind:routine` lists it and `op:read,
  path:"routine/zirkle-debug.md"` returns it.
- `core/tests/test_debug_mode.py` — six tests, two fixtures. `just test`'s
  per-file loop over `core/tests/test_*.py` picks it up with no justfile change.

### The two fixtures, and why there are two

A foreground `zirkle run` is held open by a Lua service calling
`os.execute("sleep 120")`. That is what proves debug mode answers against the
*running* host while it is busy — but `os.execute` holds the mlua interpreter,
so in that host `zirkle reload` and `zirkle call` deadlock and no event can be
provoked. The second fixture is a `zirkle daemon` over a cartridge whose service
calls `ctx:send("ping", {})`, which is what shows the tap recording and stopping.

### Verified end to end on the real default profile

A `target/debug/zirkle daemon` in this repo, then from a second shell: `debug on`,
`call tool.shell` with a context, `debug` → 7 lines in `.zirkle/logs/debug.log`,
including the finished command with its cwd, exit and output and the raw `pty`
output chunks; then `debug off` with the byte count unchanged. The wrapped shell
ran the command and returned to its prompt, untouched.

This settles the two constraints in the Outcome — both are missing arguments,
not socket limits, and neither needs a change:

- `zirkle call tool.shell '{"op":"call","input":{"command":"…"},"context":{…}}'`
  runs in the user's shell. Without `context` it answers `invocation context
  required` (`builtin/pty/tool.rs:105`, a policy check on `args["context"]`).
  Note the op is `call` with a nested `input` object, not `run`.
- `zirkle call memo '{"op":"types","cwd":"<repo>"}'` answers. Without `cwd` it
  answers `trusted memo cwd required` (`builtin/memo/src/service.rs:346`); the
  service takes cwd from the caller by design.

### What is left

1. `core/main.rs::ask` exits 0 printing nothing when the host closes the
   connection before answering — the loop just ends. From the shell that reads
   as "the command silently did nothing". Make it exit non-zero with a message
   naming the profile when the stream ends with no answer; this is shared by
   `call`, `reload`, `status` and `debug`, so fix it in `ask`, not per command.
2. Run the gates: `just check` and `just test`. The probe ran `cargo fmt`,
   `cargo clippy -p zirkle --all-targets -D warnings` and the two python files
   only.
3. One line in `core/README.md` beside the other CLI verbs, saying debug mode is
   a host-held socket setting and pointing at `routine/zirkle-debug`.

### Trap

`.cargo/config.toml` points every worktree at one shared `target/`, and every
one of them links its binary to the same `target/debug/zirkle`. A sibling lane's
build silently replaces it, and a *successful* `cargo build` will not relink
because cargo thinks the artifact is fresh — the symptom is `error:
unrecognized subcommand 'debug'` from a binary you just built. `touch
core/main.rs` before building forces the relink. The same sharing produces the
phantom `E0599: no method named inventory found for &zirkle::sdk::Host`; `touch
core/sdk.rs` clears that one. `core/tests/test_debug_mode.py` honours
`ZIRKLE_BINARY` so a pinned copy can be used while siblings build.

Stray hosts are the other trap: a previous run's socket file survives its
process, so a `zirkle debug` against a dead socket answers `Connection refused`.
`pkill -f 'target/debug/zirkle'` between probe rounds.


## Observed

Re-run after the `momo` → `zirkle` rename landed on the trunk (`c7a0e0d`) and this
lane was rebased onto it. Everything below is the renamed surface: `zirkle debug`,
`.zirkle/logs/debug.log`, `routine/zirkle-debug`. The binary is the lane's own
`/Users/feb/dev/sys/target/debug-mode-gate/debug/zirkle`, built here with an
isolated `CARGO_TARGET_DIR` — cartridge executables resolve beside it, so it is
the binary and not a copy.

Boxes 1, 3, 5, 7 and 8, against a `zirkle daemon` on the **real default profile**,
one round:

```
--- debug (initial)
{"debug":{"bytes":0,"log":"…/.zirkle/logs/debug.log","on":false}}
--- debug on
{"debug":{"bytes":0,"log":"…/.zirkle/logs/debug.log","on":true}}
--- tool.shell without context
{"data":{"content":"invocation context required","error":true},"reply":1}
--- tool.shell with context
{"data":{"content":"$ echo probe-ok\nprobe-ok\n…\n$ exit_code: 0\n","error":false},"reply":1}
--- memo list routine | grep -c zirkle-debug
1
--- debug (report)
{"debug":{"bytes":1005,…,"on":true}}
--- wc -l log
       6 .zirkle/logs/debug.log
--- debug off
{"debug":{"bytes":1005,…,"on":false}}
--- debug off again (no preceding on)
{"debug":{"bytes":1005,…,"on":false}}
--- status
{"status":[{…"name":"root","state":"Active"…},{…"name":"memo","provide":["memo","tool.memo"],"state":"Active"…},…
```

Box 4, the tap, on the same host: reported `bytes` equalled `wc -c` at every
report (`file=1005 reported={"bytes":1005…}`, and `2039 = 2039` at the end). A
`tool.shell` call **while off** left the file at 6 lines — it stops growing. Each
new line parsed as a JSON object (`dict ['data', 'event']`). Entering twice does
not double: one `on` then one command added 8 lines, two `on`s then the same
command added 6 — pty chunking makes the live count noisy, so the deterministic
evidence is the fixture `TheTap.test_entering_twice_keeps_one_tap`.

Boxes 2, 3 and 5 are also the six fixtures, all `ok` on the renamed tree:
`AgainstAForegroundRun.test_enter_report_and_leave`,
`test_leaving_without_entering_is_not_an_error`,
`test_the_run_survives_the_round_trip`, `TheTap.test_events_are_recorded_only_while_on`,
`test_entering_twice_keeps_one_tap`, `test_reported_size_matches_the_file` —
`Ran 6 tests … OK`, with `test_run_socket.py` `Ran 2 tests … OK`.

Box 6, the one defect this pass fixed, seen red then green on the same binary
against a socket that accepts a connection and closes it without answering. The
red run is from before the rebase, with the guard reverted and rebuilt; the green
run is the renamed binary:

```
=== RED RUN ===            (the `ask` guard reverted, rebuilt)
debug   exit=0 stdout='' stderr=''
status  exit=0 stdout='' stderr=''
call    exit=0 stdout='' stderr=''
=== GREEN RUN ===          (guard restored; re-run after the rename)
debug   exit=1 stderr='no debug from …/.zirkle/default: the host closed the connection'
status  exit=1 stderr='no status from …/.zirkle/default: the host closed the connection'
call    exit=1 stderr='no reply from …/.zirkle/default: the host closed the connection'
```

### Box 9, the gates

`just check` is green, exit 0 — `memory-check` now resolves `zirkle`:

```
$ just check
… cargo fmt --manifest-path builtin/memory/Cargo.toml --all -- --check
… cargo clippy --manifest-path builtin/memory/Cargo.toml --workspace --all-targets -- -D warnings
… cargo fmt --all -- --check
… cargo clippy --workspace --all-targets -- -D warnings
… bun run --cwd builtin/ui check   ($ tsc --noEmit -p tsconfig.check.json)
just check exit=0
```

`just test` runs every step green except `core/tests/test_router.py`, which is
broken on the trunk independently of this lane:

```
cargo nextest --manifest-path builtin/memory/…   1331 tests run: 1331 passed, 17 skipped
bun run --cwd builtin/ui test                    20 pass, 0 fail (9 files)
cargo nextest run --workspace                    249 tests run: 249 passed, 1 skipped
cargo test --doc --workspace                     ok
builtin/tools/tests                              Ran 7 tests … OK
zirkle --profile tools verify                      1 contracts passed
core/tests/test_debug_mode.py                    Ran 6 tests … OK
core/tests/test_development.py                   Ran 1 test … OK
core/tests/test_run_socket.py                    Ran 2 tests … OK
core/tests/test_shell.py                         Ran 6 tests … OK
core/tests/test_ui.py                            Ran 1 test … OK
builtin/harness/eval/eval_compaction.py --check  offline gate passed (exit 0)
core/tests/test_router.py                        Ran 13 tests … FAILED (failures=1)
```

The one failure is `LaunchedAgent.test_a_launched_agent_reaches_the_models_through_the_proxy`:

```
cartridge memory: research/the-watcher-stores-an-empty-body-as-a-document.md:
invalid frontmatter: mapping values are not allowed in this context at line 3 column 155
```

That file is `builtin/memory/.zirkle/memos/research/…` — inside the independently
versioned, gitignored memory workspace every lane symlinks to the trunk's one
copy. Its `description:` contains an unquoted `text: ""`, so YAML reads the line
as a nested mapping. Not this lane's file and not its to repair. The trunk is
worse, not better: the same three `LaunchedAgent` tests fail there
(`FAILED (failures=3)`), because the trunk's shared `target/debug` has no
`memory_cartridge` binary at all — `just memory-build` builds only `--bin memory`,
while `builtin/memory/cartridge.json` now names `memory_cartridge`.

Two traps cost a round each. The shared `target/` made
`profile_is_path_entries_only_with_one_tools_table` and
`declared_contracts_prove_the_real_cartridges` fail against a sibling lane's
stale `target/debug/harness` (`left: ["harness"]`, right with the two contract
keys); `touch builtin/harness/main.rs` and rebuild made both pass, and an
isolated `CARGO_TARGET_DIR` prevents it. And a `just test` whose `cargo nextest`
starts while the memory workspace's 1331 tests are still settling fails a handful
of `agent::loop` and `profile` tests with `X is not provided` — load contention,
green on the next run.
