---
complexity: 10
footprint:
  - src/tests
---

# spec03 — the suite that came with the copy runs here, and the binary is inert

`~/dev/sys/core/tests/` was written against the whole `sys` workspace. This
unit is the subset that is testable in a crate with no workspace, no builtin
cartridges and no memory banks — plus the one measurement the PRD closes on.

## What already stands

Eleven files under `src/tests/`: `bridge`, `cartridges`, `contracts`,
`folders`, `foreground`, `lifecycle`, `mod`, `process`, `reload`,
`rpc_contract`, `socket`, and `fixtures/`. 2,325 lines.

Dropped, because they reach outside this crate and cannot come:
`profile.rs` (`include_str!` of `../../.zirkle/default/init.lua`,
`../../builtin/policy/init.lua`, `../../builtin/harness/cartridge.json`, and
`built(&["-p", package])` against sibling workspace packages) and `policy.rs`
(`include_str!` of `../../builtin/policy/init.lua` — it tests a Lua cartridge,
not the host). `landscape.rs` went with its module. The three `bun_wire_*`
cases in `rpc_contract.rs` shell out to `../builtin/ui/tests/fixtures/rpc/`;
their `rust_sdk_*` twins cover the same three contracts, so those three and
the `"bun"` arm of `peer()` are cut. The `__pycache__` directory and eleven
stray `test_*.py` files in the original were not copied.

`tests/memory.rs` is replaced by `src/tests/reload.rs`. Every test in the old
file asserted through the bank, so none could be kept verbatim; three were
ported to assert through the generation instead, which is what the kept guard
actually promises:

- `switching_keeps_consumers_bound_and_rejects_bad_migrations` — a consumer
  that captured a service reference still reaches the new generation after a
  swap and keeps its own fiber uid; a candidate that raises never publishes;
  the transaction it failed in still finishes, so the next swap works.
- `corrected_code_can_recover_a_failed_initial_generation` — the
  inactive-entry arm of `replace_entry`.
- `two_entries_of_one_file_each_get_their_own_switch`.

`fixtures/rpc_fixture.rs` no longer checkpoints; its `reject` config now
stands alone, and `rpc_contract.rs::replacement` sets `{generation=2,
reject=true}` accordingly. The `"memory"` roundtrip arm is gone.
`mod.rs::built` still passes `-p zirkle`, which resolves in a standalone crate.

## What is left

Run the suite. Nothing here has been executed — see spec01 on the dyld stall.
`cartridges.rs` and `process.rs` spawn example binaries through `built()`, so
they are the first to fail if the machine is still wedged; that is an
environment failure, not a test failure, and `probe/dyld-hang.sh` tells the
two apart. Then take the PRD's closing measurement.

## Acceptance

- [ ] `cargo test` runs all eleven files and reports 0 failed
- [ ] `src/tests/reload.rs` passes, and no test in `src/tests/` names a bank, a snapshot or a checkpoint
- [ ] `./target/debug/zirkle --help` exits 0 and prints the subcommand list
- [ ] `--help` answers in under 3ms and under 16MB RSS on this machine, measured and written into the report
- [ ] the binary started with no profile directory loads no cartridge and provides no service

## Verify and Proof

```sh
sh .pearde/prds/the-host-binary/probe/dyld-hang.sh
cargo test 2>&1 | tail -30
grep -rn "bank\|snapshot\|checkpoint" src/tests || echo "no bank in the suite"
/usr/bin/time -l ./target/debug/zirkle --help 2>&1 | grep 'maximum resident'
python3 -c "import subprocess,time;t=[(lambda a=time.perf_counter(): (subprocess.run(['./target/debug/zirkle','--help'],capture_output=True), (time.perf_counter()-a)*1000)[1])() for _ in range(20)];print('median ms', sorted(t)[10])"
```
