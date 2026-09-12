---
kind: work
description: "One session/run/request id ties host, Lua and Bun diagnostics together, off protocol stdout, bounded and redacted"
status: done
owner: "sys-opus-2026-09-12/implementer-a-turn-carries-one-id-through-host-lua-and-bun"
level: 11
priority: P1
estimate: 1d
actual: 1d
---

# A turn carries one id through host, Lua and Bun

## Outcome

One terminal turn is one traceable thing. A session, run and request identifier
travels with the work across the three runtimes zirkle spans — the Rust host, the
Lua profile, and the Bun UI and its backends — so a diagnostic line from any of
them can be joined to the turn that produced it, and so a probe report can cite
the evidence for a specific call rather than a timestamp range.

The evidence channel that carries it: diagnostics stay off the protocol stdout
that the JSON-line wire owns, retention is bounded rather than unbounded or
truncating, and credentials and prompt/tool bodies are omitted by default.

Nothing of this exists today, verified on branch
`terminal-integration-and-native-memos`: a grep for `run_id|request_id|trace_id|
correlat` across `core/` and `builtin/` (excluding `builtin/memory` and
`builtin/proxy`) returns only prose in test names and doc comments. The wire in
`core/cartridge.rs` numbers each request with a per-link `AtomicU64` that means
nothing outside that one process pair. The whole evidence channel is `just run`
redirecting UI stderr into a single `.zirkle/logs/ui.log` (`justfile:81-83`),
truncated on every start, with no per-turn boundary. Host and cartridge stderr
is `Stdio::inherit()` (`core/cartridge.rs:242,283`), so every cartridge writes
onto the same stream.

Also confirm, and open new work only if it is false: the transcript and the
`/context` inspector stay reachable while a full-screen editor owns the shell.
`builtin/harness/README.md` documents `/context` and `Ctrl+O`, and
`builtin/ui/ui/terminal.tsx` binds `Ctrl+G` and `Ctrl+F`, but no `Ctrl+O`
binding appears in the shipped terminal feature.

This is independent of [[debug-mode-opens-and-closes-from-the-shell]] and its
successors: they need an evidence channel, they do not need this one to exist
before they work.

## Check

- [x] A socket `call` that names a `turn` reaches both a Lua service and a
      cartridge process with that same id; a `call` that names none gets one
      minted id, the same one in both places.
- [x] A call the Bun UI makes while handling a host call carries the turn that
      host call named; a host frame with no turn produces onward frames with no
      `turn` field.
- [x] Every diagnostic the host and its cartridges write is one JSON object on
      the diagnostic stream carrying `turn`, `src` and `msg`, and the protocol
      stdout of every cartridge still parses as pure wire frames.
- [x] A run that writes more diagnostic bytes than the configured cap leaves the
      diagnostic file at or under that cap; nothing truncates it at startup.
- [x] A diagnostic whose payload holds a credential or a prompt/tool body omits
      those fields by default and says it omitted them.
- [x] `zirkle call` accepts a turn and reports the one it used, so a probe report
      can cite the id of a specific call.

```sh
cd /Users/feb/dev/sys
# the id across host, Lua and a cartridge process, and across the Bun wire
touch core/lib.rs core/sdk.rs   # shared target/: clears a sibling lane's stale rlib
cargo test -p zirkle --lib -- turn
bun --conditions=browser test tests/wire.test.ts --cwd builtin/ui

# the sink: bounded retention and redaction are covered by named unit tests,
# and the filter must actually match something
cargo test -p zirkle --lib -- --list | tee /tmp/turn-tests.txt
grep -q 'bound' /tmp/turn-tests.txt
grep -q 'redact\|omit' /tmp/turn-tests.txt
cargo test -p zirkle --lib

# one real cartridge call, deliberately failing so the host reports something:
# every diagnostic line is JSON and turn-tagged, and none of it reached stdout
cargo build -p zirkle -p tools --bins
./target/debug/zirkle --profile tools run tools '"no-such-command"' \
  >/tmp/turn-stdout.txt 2>/tmp/turn-diag.jsonl || true
python3 -c "
import json
rows=[json.loads(l) for l in open('/tmp/turn-diag.jsonl') if l.strip()]
assert rows, 'no diagnostics were written'
assert all(set(('turn','src','msg')) <= set(r) for r in rows), rows[:3]
print(open('/tmp/turn-stdout.txt').read())"

just check && just test
```

## Approach

An analyst pass probed this on lane
`work/a-turn-carries-one-id-through-host-lua-and-bun`, commit `e6604b9`. The
probe built the first half of the Outcome and it works; the second half — the
evidence channel — is what is left.

### What the probe established

Every claim in the Outcome re-verified on this branch. `grep -rE
'run_id|request_id|trace_id|correlat' core builtin` (less `memory` and `proxy`)
returns only test names, doc comments and old memo prose — no identifier. The
`just run` redirect is `justfile:86`, `2>.zirkle/logs/ui.log`, truncating; host
and cartridge stderr is `Stdio::inherit()` at `core/cartridge.rs:242,283`.

The turn does not need inventing per runtime, because every cross-runtime call
already funnels through two places: `Host::invoke`/`invoke_raw`
(`core/lua.rs:316,327`) reaches a Lua function, a process `Remote` or plain
data, and `Link::request` (`core/cartridge.rs`) is the one place an outgoing
frame is numbered — and both ends of the wire use the same `Link`, so one stamp
covers host→cartridge and cartridge→host. The turn is therefore ambient, not a
parameter: a `tokio::task_local` in the new `core/turn.rs`.

Committed in the probe and passing:

- `core/turn.rs` — `current`, `mint`, `of(frame)`, `stamp(frame)`, `scope`.
- `Link::request` stamps the ambient turn onto every outgoing frame.
- `cartridge::handle` (the `call` branch), `sdk::Host::dispatch` and
  `socket::client` re-enter the turn around their `tokio::spawn`, which does not
  inherit a task-local. `socket::client` adopts a client's `turn` or mints one.
- `zirkle.turn()` in Lua, `zirkle::sdk::Host::turn()` in a Rust cartridge.
- `builtin/ui/src/wire.ts` carries it in an `AsyncLocalStorage`: `receive` runs
  each handler in the frame's turn, `request` stamps it, and `turn()` reads it.
- Tests: `one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua`
  (`core/tests/socket.rs`, with a `turn` op added to the rpc fixture) proves
  socket → host → cartridge process → back into the host → Lua all name one id;
  `builtin/ui/tests/wire.test.ts` proves the Bun hop and that an untagged frame
  stays untagged. `cargo test -p zirkle --lib` and the Bun suite are green.

Two facts the probe found that the Outcome's remaining half rests on:

- The UI cartridge's stdout is the wire, but its terminal is a separate tty
  opened by `openTerminal` (`builtin/ui/src/main.tsx`), so stderr is free for
  diagnostics in all three runtimes with no rendering conflict.
- The Outcome's own constraints settle where the sink lives: redaction and a
  byte cap are only possible if the host formats and counts the writes, so an
  inherited raw stderr cannot be the channel. Hence the step below.

### What is left

1. `core/turn.rs` gains `diagnostic(src, msg, fields)`: one JSON line,
   `{"t","turn","src","msg",…}`, to the host's diagnostic sink. Redaction is a
   field allowlist — anything named like a credential, prompt, or tool body is
   replaced by `"<omitted>"` and counted in an `omitted` list. `Host::report`
   (`core/lua.rs:112`) becomes its first caller; keep the outbox JSON it already
   sends unchanged so socket clients do not break.
2. The host owns every diagnostic byte: change `Stdio::inherit()` to
   `Stdio::piped()` at `core/cartridge.rs:242,283` and read each cartridge's
   stderr line by line into `diagnostic(<cartridge name>, line)`. Lines a
   cartridge already emits as JSON pass through with `turn` merged in; anything
   else becomes `msg`.
3. Bound it. The sink writes through a byte counter; at the cap it renames the
   file to `.1` (one generation kept) and reopens. Two generations of a
   configured `max_bytes` is the whole retention policy — no startup truncation.
4. `builtin/ui` writes its diagnostics as JSON lines on stderr through a small
   helper beside `turn()` in `src/wire.ts`, so step 2 collects them tagged.
5. `justfile:86` stops redirecting: `zirkle run` opens `.zirkle/logs/zirkle.jsonl`
   itself. Keep the failure `tail` for the exit path.
6. `zirkle call` takes the turn (`--turn`, else minted) and prints it with the
   reply, so a probe report can cite one call.

### The confirm item, and it is false

`agent {op:"context"}` exists (`builtin/agent/lib.rs:264`) and
`builtin/harness/README.md` documents both `/context` and `Ctrl+O`, but the
shipped terminal feature has neither: `builtin/ui/ui/terminal.tsx` binds only
Ctrl+G, Ctrl+F and Ctrl+Y, and no slash-command parsing exists anywhere in
`builtin/ui/`. The inspector has no entry point at all, full-screen editor or
not. That is separate work, recorded as
[[the-context-inspector-opens-from-the-chat-editor]]; it needs a `subwork:`
attach from the next pass.

## What landed, and what the boxes were observed on

Lane `work/a-turn-carries-one-id-through-host-lua-and-bun`, rebased onto
`terminal-integration-and-native-memos` after `c7a0e0d` renamed `momo` to `zirkle`
across the record, the crate and the cartridges. `458121d` is the analyst's
probe; the evidence channel is `1a373a3`, `7a7546c`, `7d9529e`, `eacd5ee`,
`eba7037` and `dbc2623`. The rebase kept both sides' intent: the Lua helper is
`zirkle.turn()`, the sink reads `ZIRKLE_DIAGNOSTICS` and `ZIRKLE_DIAGNOSTICS_MAX_BYTES`,
and `justfile:86` writes `.zirkle/logs/zirkle.jsonl`.

The sink is `turn::diagnostic(src, msg, fields)` in `core/turn.rs`. Default
stream is stderr; `ZIRKLE_DIAGNOSTICS` names a file, opened for append, and
`ZIRKLE_DIAGNOSTICS_MAX_BYTES` (default 8 MiB) caps it with one rotated `.1`
generation. `justfile:86` sets that variable instead of redirecting, so `zirkle
run` opens the file itself; the failure `tail` stays.

Every box was observed on a binary built from this lane into
`CARGO_TARGET_DIR=/Users/feb/dev/sys/target/turn-id-gate` — the default shared
`target/` is clobbered continuously by fifteen sibling lanes, which uplift their
own `zirkle`, `harness` and `libzirkle.rlib` over this one.

**One id across socket, host, cartridge process and Lua** —
`cargo test -p zirkle --lib -- turn`:

```
test turn::tests::a_credential_or_a_prompt_body_is_omitted_and_named ... ok
test turn::tests::the_sink_stays_bounded_by_rotating_one_generation ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test result: ok. 5 passed; 0 failed
```

The socket test asserts both halves: a client naming `turn-probe-1` gets
`{"cartridge":"turn-probe-1","lua":"turn-probe-1"}`, and a client naming none
gets one minted id that is the same string in both places.

**The Bun hop** — `bun --conditions=browser test tests/wire.test.ts`:
`2 pass, 0 fail`. The test reads `turn()` inside a handler entered from a frame
tagged `turn-probe-1`, finds the onward `pty` call stamped with it, and asserts
an untagged frame produces an onward frame with no `turn` property at all.

**JSON lines, and protocol stdout untouched.** A throwaway cartridge that writes
one plain and one JSON line to stderr, run through the built `zirkle`:

```
{"msg":"running","src":"development","t":…,"turn":null}
{"msg":"noisy: a plain stderr line","src":"noisy","t":…,"turn":null}
{"api_key":"<omitted>","level":"warn","msg":"already JSON","omitted":["api_key"],"src":"noisy","t":…,"turn":null}
{"msg":"stopped","src":"development","t":…,"turn":null}
```

stdout over the same run was exactly `"ok"` — the service reply, nothing else.
The memo's own probe (`zirkle --profile tools run tools '"no-such-command"'`)
writes three rows, all carrying `turn`, `src` and `msg`, with an empty stdout:

```
{"msg": "running", "src": "development", "t": …, "turn": null}
{"msg": "chat failed", "src": "development", "t": …, "turn": null}
{"msg": "unknown variant `no-such-command`, expected one of `bundle`, `lane`, `land`, `lane-rm`", "src": "zirkle", "t": …, "turn": null}
```

**The cap, and no startup truncation.** With
`ZIRKLE_DIAGNOSTICS_MAX_BYTES=4096` and a cartridge spamming 200 stderr lines:
live generation `204` bytes, rotated `.1` generation `4046` bytes — both at or
under the cap, and the process's raw stderr was `0` bytes because the host owns
every one of them. With a cap the run does not reach, a line written into the
file *before* the run is still line 1 afterwards
(`{"msg":"a line written before the run started","src":"probe"}`, 203 lines in
the file), so the sink appends and never truncates.

**Redaction.** `api_key` above became `"<omitted>"` and is named in
`"omitted":["api_key"]`. The name list covers credentials (`key`, `token`,
`secret`, `password`, `credential`, `auth`, `cookie`), prompt bodies (`prompt`,
`message`, `completion`, `content`) and tool bodies (`args`, `input`, `output`,
`result`, `body`), matched as a lowercased substring and applied recursively.

**`zirkle call` names its turn**, against a live `--profile tools` daemon:

```
$ zirkle --profile tools call tools '"lane-rm nonexistent-lane"' --turn probe-42
{"error":"no registered lane","reply":1,"turn":"probe-42"}
$ zirkle --profile tools call tools '"lane-rm nonexistent-lane"'
{"error":"no registered lane","reply":1,"turn":"18d483c201526668-1"}
```

### The gates

`just check` passes, exit 0 — `memory-check`, `cargo fmt --check`,
`cargo clippy --workspace --all-targets -D warnings`, `bun install
--frozen-lockfile` and `bun run check` all green.

`just test` still dies in `memory-test`, now for a reason inside the separate,
mid-migration `builtin/memory` repository: `tests/cartridge.rs` calls
`zirkle::lua::Host::new`, but `zirkle` is declared in that crate's `[dependencies]`
only, not its `[dev-dependencies]`, so an integration test cannot link it —

```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `zirkle`
   --> tests/cartridge.rs:173:13
error: could not compile `memory` (test "cartridge") due to 8 previous errors
error: recipe `memory-test` failed on line 57 with exit code 101
```

Every other step of `just test` was run in this lane and is green:
`bun test` (21 pass), `cargo nextest run --workspace` (252 tests, 251 passed,
1 skipped — the one red was `pty::process::shell_serializes_input_…`, a timeout
that passes alone), `cargo test --doc --workspace`, `cargo test -p zirkle --lib`
(70 passed), `builtin/tools/tests` (7 OK), `test_development.py` (OK),
`test_shell.py` (6 OK), `test_ui.py` (1 OK), and `eval_compaction.py --check`
(offline gate passed).

`core/tests/test_router.py` is down to one failure, also from the memory
migration: after building that repo's new `memory_cartridge` bin by hand, two of
the three clear, and the last reports `cartridge memory:
research/the-watcher-stores-an-empty-body-as-a-document.md: invalid frontmatter:
mapping values are not allowed in this context at line 3 column 155` — a
malformed memo in the memory repository's own record.

`core/tests/test_development.py` matched the old `development: <phase>` prose
and the `Development rebuild` banner; `eba7037` moves it onto the JSON object
prefix, and it passes.

### One thing deliberately left as prose

`zirkle`'s own argument and connection errors — an invalid JSON argument, an
unknown `--yolo` placement, no daemon on the socket — stay on plain stderr. They
answer the caller, not a turn, and routing them to a configured sink file would
make the CLI silent when it refuses.
