---
kind: work
description: "Repository CI runs the gates and reports terminal coverage explicitly"
status: open
level: 10
priority: P1
estimate: 4h
needs:
  - "[[fresh-checkouts-can-run-the-gates]]"
---

# Repository CI runs the gates and reports terminal coverage explicitly

## Outcome

Checked-in CI exercises the supported host platforms and language boundaries with the same commands used locally. The first matrix covers Linux and macOS with Bash/Zsh and Neovim; additional shells are explicitly classified as required or optional.

## Spec

Files (all committed on the lane `work/ci-proves-the-supported-terminal-matrix`):

- `.github/workflows/ci.yml` — pull-request + `main` push CI, matrix `ubuntu-latest` and `macos-latest`. Steps in order: checkout; bootstrap `builtin/memory` by cloning `https://github.com/yesitsfebreeze/kern.git` and checking out the pinned revision `58850707f37f7cd10939b742c4c9898e84552193` (fresh checkouts have no `builtin/memory` — it is a separately versioned repo ignored by the root); `dtolnay/rust-toolchain@stable` with `rustfmt, clippy`; `oven-sh/setup-bun@v2`; `actions/setup-python@v5` (3.12); `taiki-e/install-action@v2` installed with `tool: nextest`; install the required fixtures (ubuntu: `apt-get install -y zsh neovim`; macos: `brew install neovim`; bash and zsh already ship on both); then `just fixtures`, `just check`, `just test`, `just fail-probe`; a final `if: failure()` step reprints `just fixtures` so failure logs carry environment versions on top of the failing step's own output (step names name the language boundary).
- `scripts/ci-preflight.sh` — one classification and version report, shared by CI and local: prints os/rust/cargo/bun/python versions, asserts the required matrix `bash zsh nvim` (missing any one exits 1 naming it), counts the optional fixture `nu` as a skip when absent, and mirrors everything into `$GITHUB_STEP_SUMMARY` when that variable is set.
- `scripts/ci-fail-probe.sh` — self-proves the failure path: plants `core/tests/test_ci_failing_fixture.py`, runs the exact per-file Python gate from the `just test` loop (`python3 -m unittest discover -s core/tests -p test_ci_failing_fixture.py -q`), requires a non-zero exit, then removes the file and asserts it is gone.
- `justfile` — two recipes so CI and local share the same commands: `fixtures` (`@bash scripts/ci-preflight.sh`) and `fail-probe` (`@bash scripts/ci-fail-probe.sh`).

Steps:

1. `actionlint .github/workflows/ci.yml` — the workflow is valid (clean on the probe: no errors).
2. `just fixtures` — prints the environment and fixture report; exit 0 with all required present.
3. `just fail-probe` — the planted failing fixture makes its per-file gate fail and is then removed.
4. `just check` and `just test` — the gate steps the workflow invokes, run against the lane (same commands, same pinned memory revision). `just test` already runs each `core/tests/test_*.py` suite in its own process.

Probe (analyst, lane commit `97288b8`): the workflow, both scripts and the two justfile recipes are written and committed; `actionlint` clean; `bash scripts/ci-preflight.sh` exits 0 on the full local matrix and exits 1 naming `nvim` with `PATH="/usr/bin:/bin"` while counting the optional `nu` skip; `just fail-probe` passes; `just fixtures`/`just fail-probe` run through the recipes; the Check's `sh` block passes verbatim. `just check` is green on the lane. `just test` was attempted three times on the lane: attempt 1 hit the kern e2e reaper flake, attempt 2 the momo profile-contract flake (each passes in isolation), attempt 3 a resolution failure because the shared `builtin/memory` kern working tree is being concurrently edited by another lane (`Cargo.toml` now adds `zirkle = { path = "../../core" }`) — the probe touches no kern file. The workflow gates with the `just` recipes, so the remaining defined work is a clean `just check`/`just test` once the kern tree settles, then ticking the boxes. The workflow's install steps (apt/brew, nextest, pinned clone) execute only in the action runner.

## Check

- [ ] `actionlint .github/workflows/ci.yml` reports no errors; the workflow runs on `ubuntu-latest` and `macos-latest` and gates with the same commands used locally (`just check`, `just test`, `just fixtures`, `just fail-probe`) after bootstrapping `builtin/memory` from the pinned kern revision `58850707f37f7cd10939b742c4c9898e84552193`.
- [ ] `bash scripts/ci-preflight.sh` exits 0 with bash/zsh/nvim present and prints a fixture report; with a required executable hidden from PATH it exits non-zero and names it, while an absent optional fixture counts a skip (`optional_skips=1`) without failing.
- [ ] `bash scripts/ci-fail-probe.sh` succeeds: it plants a deliberately failing test in `core/tests`, the per-file Python gate exits non-zero (one process per file fails loudly), and the planted file is gone afterwards — `ls core/tests/test_ci_failing_fixture.py` fails.

```sh
actionlint .github/workflows/ci.yml
bash scripts/ci-preflight.sh
bash scripts/ci-fail-probe.sh
# a missing required executable must fail the preflight, naming it
PATH="/usr/bin:/bin" bash scripts/ci-preflight.sh && exit 1 || test $? -eq 1
```

## Approach

Observed 2026-09-12: No tracked `.github` workflow was found. `justfile` already defines the local gates. `builtin/pty/tests/process.rs` prints a skip and returns successfully when some shells or nvim are absent. External CI settings were not inspected.

Audit: [[runtime-audit-2026-09-12]].
