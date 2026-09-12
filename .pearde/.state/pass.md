# Pass — pass 2 on cartridge: Q1 answered, telemetry PRD opened, the-host-binary specced, 3 forks put

## Established
- **`/Users/feb/dev/cartridge` is not a git repo, and there is no repo above it.**
  `/Users/feb/dev` and `/Users/feb` both have no `.git`; `~/dev/sys` — where
  `src/` was copied from — is one. Re-verified 15:31. Every `claim` prints `no
  baseline — not inside a git repo` and moves the state anyway. No lane, no
  baseline, no revert, and two passes of uncommitted work now stand in the tree
- **`syspolicyd` is still pegged at 100% CPU** — checked 16:12, `ps` shows pid 498.
  While it is, nothing newly linked will start, so there is **no compile signal on
  this machine**. hostsmith re-tested three workarounds this pass and all three
  failed; it wrote that up at `sources/260912-fb23.md`. Repro at
  `.pearde/prds/the-host-binary/probe/dyld-hang.sh`
- Q1 the-host-binary **answered** 15:26. The user deferred ("what you think is
  best"), so the recommended option was taken and recorded in their words:
  *separate them — the swap guard stays, only the stored data nothing has asked
  for yet goes*
- hostsmith returned **SPECCED** · 3 specs (8/14/10, sum 32) · complexity 30 ·
  blast-radius high · workflow `probe-then-spec` · 15 boxes ·
  `.pearde/prds/the-host-binary/report.md` · collected 16:05. The answer is
  **applied in the tree**: `src/memory.rs` split, `src/reload.rs` new (gate,
  epoch, pending only), the bank and its nine call sites deleted, `fiber.rs`
  byte-identical, `runtime.rs` eight renamed lines plus one deletion, tests moved
  to `src/tests/` so `mod tests;` resolves
- **New PRD from the user, not from a build**: `the-telemetry-channels`, p72,
  `needs: the-wire`. Their requirement is quoted **verbatim** in its body — a
  base-layer publish/subscribe event stream, replayable, deterministic, accurate,
  fast, small, for agentic swarm work. Added to `vision.md` as a terminal with the
  edge `the-wire -> the-telemetry-channels`, so scan reads it on-axis. Board is
  now **10 PRDs** · 15:28
- **`questions broken` is cleared.** the-manifest's `## Questions` was the user's
  own prose with one embedded recommendation. Rewritten into drill format, three
  genuinely different outcomes, technical anchor in the HTML comment.
  `pearde questions` is silent · 15:33
- **The board's workflow library was empty**; `probe-then-spec` plus its eight
  atomics are now its first entries, drafted from the run hostsmith actually took.
  `pearde workflow check` clean. All at `runs: 0` — the implementer's collect is
  their `runs: 1` · 16:04
- Two grammar rows written, `stream` and `channel`, and the `wire` row reworded
  from "JSON-lines channel" to "JSON-lines connection" · 15:38
- `pearde knowledge round`: every tool current (scout 7d, graph 0d, vault 0d) —
  nothing owed at step 7. `pearde knowledge query` run on both the manifest fork
  and the git fork; neither is on record, both gaps enqueued · 15:35
- `pearde doctor`: `index` (3) and `claims` (11) are **pearde's own install**
  under `/Users/feb/dev/infra/pearde`, not this board's to fix; `plugins off` is
  an opt-in. None gates dispatch · 15:37
- Agent type `pearde-analyst` resolves to `auto:free`, unreachable on this
  account. **Every dispatch must carry `model: opus`** — carried from pass 1

## Decided
- Took the recommended option on Q1 rather than putting the fork back. The user
  said "what you think is best"; a second ask on an answered question spends a
  round on nothing
- Filed the telemetry requirement as **its own PRD**, not as scope bolted onto
  [[the-wire]]. The wire is one node talking to the dependency that launched it;
  telemetry is many nodes broadcasting to whoever listens. Widening the wire's
  contract is the thing REFINE exists to undo
- Named it **the-telemetry-channels** and called its log the **stream**, never a
  ledger. `ledger` is already the registry of installed cartridges on this board;
  two meanings for one word in one vision is a defect the analyst would have hit
- Left the broker question *inside* the telemetry PRD as scope to settle, not as
  a drill fork. Whether events travel along birth pipes or through a
  host-resident bus is what an analyst's build finds out; the vision's "no
  discovery, no broker" is the constraint it argues against
- Wrote the-manifest's three answers rather than sending an analyst back — the
  loop offers both, the PRD is gated so no analyst could go, and the fork was
  already fully stated by the user. Only the format was unaskable
- **Did not dispatch the implementer on the-host-binary, though it is ready.**
  Every one of its three specs opens with `cargo build` as acceptance box one, and
  with `syspolicyd` pegged that box cannot be ticked. A worker sent now returns
  BLOCKED on box one, having rewritten `src/` into a tree with no version control
  to revert it. Both walls are Q1 and Q2 below, and both are asked *on this PRD*,
  so it is the asker and gated by its own questions — and every other PRD
  `needs:` it. Nothing was dispatchable past the drill
- **My own error, recorded so the next pass does not repeat it:** I collected
  the-host-binary at 15:52 off a report file that said `Verdict: QUESTION`, while
  the worker was still writing it — the file existed and the transcript had gone
  quiet for one interval. It finished SPECCED. Recovering cost `set open`,
  re-`claim`, re-`collect`. **A report file appearing is not a finished worker.**
  Wait for the agent's own return line, or for the transcript to be flat across
  several intervals, before collecting
- Ruled on hostsmith's out-of-scope findings: two were already fixed by this pass
  (empty workflow library, the-manifest's malformed fork); the rest — the crate
  still being named `zirkle`, `kache` as `rustc-wrapper`, `src/main.rs` kept off
  the keep list because a binary needs a `main` — are notes, not derived PRDs

## Asked
- Q1 the-host-binary · answered 15:26 · closed
- Q1 the-host-binary · whether anything built here can be checked · out 16:15
- Q2 the-host-binary · where this project's history lives · out 16:15
- Q3 the-manifest · what an inner cartridge offers the outside · out 16:15

## Owed
- **Q1 and Q2 are board-level and are deliberately not written into any
  `prd.md`** — writing them there would park the-host-binary in `question` and
  strand its specs. They live in `.pearde/.state/ask.md` only, so `pearde answer`
  does not apply to them: the next pass **acts** on those two answers directly.
  Q3 *is* in `the-manifest/prd.md` and takes `pearde answer the-manifest Q1 "…"`
- On Q1 *restart*: re-check `ps -Ao pid,pcpu,comm | grep syspolicyd`, then claim
  the-host-binary to an implementer and dispatch. On *carry on*: dispatch anyway
  and expect box one to stay open. On *build elsewhere*: that is a new PRD
- On Q2 *its own history*: `git init`, one baseline commit of everything standing
  — the trim, `reload.rs`, `src/tests/`, the probe — **before** any implementer
  writes, then `pearde session take` starts working and lanes become real
- the-host-binary is `specced`, ready, 0/15 boxes. Nothing else on the board is
  dispatchable until it is `done`; all nine others `needs:` it, directly or down
  the chain
- Every dispatch carries `model: opus`
