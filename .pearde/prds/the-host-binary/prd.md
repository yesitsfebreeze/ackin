---
state: done
origin: requested
priority: 90
complexity: 32
blast-radius: high
workflow: probe-then-spec
needs:
  - the-build-signal
actual: 0.02h
---


# The host binary

`src/` in this repo is a byte-identical copy of `~/dev/sys/core/*.rs` — the
zirkle core crate — with no `Cargo.toml`, no `tests/` and no `README.md`. It
does not build: `lib.rs` declares `mod tests` against a directory that was not
copied. Make it a crate of its own, and the smallest one that still holds the
mechanism.

Keep, because it is proven and it is the asset: `runtime.rs` and `fiber.rs`
(the dependency graph, revertible effects, the lifecycle guard — 1,053 lines),
`cartridge.rs` (the process wire), `sdk.rs`, `socket.rs`, `lua.rs`,
`context.rs`, `loader.rs`, `turn.rs`.

Cut, until a cartridge asks for it back: `development.rs` (cargo
rebuild-on-save, a `sys` workflow, not a property of this system),
`landscape.rs` (a read-only introspection projection built for that repo's memo
tool), `memory.rs` (host-resident memory banks — nothing here needs them yet).

Bring over `~/dev/sys/core/Cargo.toml` and `~/dev/sys/core/tests/`, minus what
the cut modules covered.

The language question is closed and does not reopen inside this PRD. Rust,
because the host embeds Lua (a C library), applies OS sandbox policy to its
children through syscalls, stays resident for days without leaking, and is the
one component in this design that is never rewritten on a whim — choosing a
scripting language would buy iteration speed on the file touched least.
Measured on this machine, and re-measured against the built binary on
2026-09-12: the debug build answers `--help` in **3.07ms wall / 2.07ms net** at
**8.4MiB RSS**, against Bun 8.9ms/22.9MB, Deno 12.1ms, Python 15.6ms, Node
24.0ms. The conclusion is unchanged and the language question stays closed —
even the gross figure is a third of Bun's and a seventh of Node's, and the RSS
claim reproduced almost exactly.

The `0.9ms` this paragraph carried until now did **not** reproduce, and the
correction matters more than the number. On this host `fork`/`exec` alone costs
~1.0ms: `/usr/bin/true`, which does nothing, times at 1.001ms over 200 runs. So
0.9ms is below the floor any binary here can reach, and the method behind it was
never recorded. `2.07ms` is the binary's own cost with that floor subtracted;
`3.07ms` is what a reader timing it at a shell will see. Anything comparing
these five runtimes must subtract the same floor from all five or compare none
of them — see `probe/help-latency.sh`, which measures subject and floor on one
loop.

Must not change: the semantics of the graph in `runtime.rs` and `fiber.rs`.
Trimming is deletion of whole modules, not redesign of what remains.

At the end there is a binary that builds, starts in about a millisecond, and
does nothing whatsoever on its own.

## Questions

### Q1: What happens to the cartridge memory banks

The part you asked to cut also holds the guard that lets a running cartridge be
swapped for a new one without dropping calls already in flight. Cutting the
whole part takes that guard with it, so the two have to be separated or kept
together?

1. **Separate them** — the swap guard stays, and only the stored data nothing has asked for yet goes. (recommended)
2. **Keep both** — nothing is removed here; the stored data stays until something asks for it back.
3. **Cut both** — stored data and live swapping both go, and reloading a cartridge means restarting the host.

<!-- for the board: src/memory.rs is Snapshot/read/update (the bank) fused with gate/epoch/pending/begin/finish/fork/thaw (the reload transaction). Bank callers: src/sdk.rs memory()/checkpoint(), src/cartridge.rs memory op, src/context.rs ctx:memory/ctx:checkpoint, src/runtime.rs Ctx::memory, tests/memory.rs. Gate callers: src/loader.rs replace_entry + request_reload, src/service.rs whole module, src/lua.rs to_lua guard, src/runtime.rs on()/provide() resident wrapping, src/fiber.rs Service downcasts. Answer lands in the-host-binary spec01. -->

## Answers

**Q1** *(answered 2026-09-12 15:17)* — Separate them — the swap guard stays, and only the stored data nothing has asked for yet goes.

## Board notes for the implementer *(rewritten by the pass, 2026-09-12 16:5x)*

**The wall is down. The compiler works. Everything in this PRD is now
verifiable, and nothing in it may be ticked on reasoning any more.**

The user restarted the stuck daemon themselves. `syspolicyd` is pid 93109 at
0.0% CPU; the old pid 498 is gone. `probe/dyld-hang.sh` prints `exit=0`, and
[[the-build-signal]] — the PRD that held this one — is `done` with a capped
four-stage gate on disk. Every spec here now opens with that gate as box one:

```sh
sh .pearde/prds/the-build-signal/probe/build-signal.sh . | tail -1   # signal=live
```

Run it first. It costs twenty seconds and it is the only thing that tells a
broken machine from a broken crate — on a re-pegged machine `cargo` does not
fail, it hangs at 0% CPU, and you would spend an afternoon before noticing.

**What the last run left you.** hostwright ticked 7 boxes honestly and left 8
open because it had no compiler: the `cargo build` box in spec01 and spec02, and
all five of spec03. It also left one correction already applied — `runtime.rs`
differs in **7** renamed lines, not 8, plus the 10-line `Ctx::memory` deletion,
which `diff` confirms as 17 removed against 7 added. Its report is on disk and
its diagnosis does not need repeating.

**Your job is the 8 open boxes, and they are all runnable now.** `cargo build
--all-targets`, `cargo test`, `./target/debug/zirkle --help` with its timing and
RSS measured and written into the report, and the no-profile-directory check.
With `warnings = "deny"`, expect the first build to surface orphaned imports and
now-dead private functions left by the cut — fix them; that is the compiler
doing the half of the standing direction that `grep` cannot.

**Standing direction from the user, verbatim:** *"This tree is a plain copy. We
need to remove everything that is not currently used by anything in the current
tree."* Two things follow, and the second is new work:

- Every `Cargo.toml` dependency was checked and **all are reached** — `notify`
  from `loader.rs`'s watcher, `tempfile` from `src/tests/`, the rest throughout.
  **Drop no dependency**, and the box pinning `Cargo.toml` to the original minus
  four `path =` values stands.
- **`sdk::on_reload` at `src/sdk.rs:58` has no caller anywhere in this tree.**
  The last run reported it and did not delete it, which was right with no
  compiler. You have one. Delete it, build, and let the compiler prove the cut
  is safe — and do the same for anything else the build shows is reachable from
  nothing. If a `pub` item has no caller but deleting it breaks the build, say
  so and keep it; that is a finding, not a failure.

**This tree has its own history.** `git init` ran at the root, `main` is the
branch, and your lane is a real branch off it. Work can be reverted.

**Say plainly what you did and did not verify.** The machine is healthy now, so
an open box needs a reason that is not the machine.

## History

**failed, retried 2026-09-12 17:18**

spec01: exit 0
hi
exit=0  (0 healthy, 142 = hung in dyld)
   Compiling notify-types v2.1.0
   Compiling rustc-hash v2.1.3
   Compiling bytes v1.12.1
   Compiling log v0.4.34
   Compiling either v1.18.0
   Compiling getrandom v0.4.3
   Compiling rustix v1.1.4
   Compiling xxhash-rust v0.8.18
   Compiling itoa v1.0.18
   Compiling tokio v1.53.1
   Compiling clap v4.6.6
   Compiling fastrand v2.5.0
   Compiling once_cell v1.21.4
   Compiling futures-executor v0.3.34
   Compiling futures v0.3.34
   Compiling serde-value v0.7.0
   Compiling tempfile v3.27.0
   Compiling mlua v0.12.1
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s6052)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.42s

spec02: exit 1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
bank is gone
fiber identical
10c10
< use tokio::sync::{broadcast, mpsc, watch};
---
> use tokio::sync::{broadcast, watch};
59c59
< 	pub memory: crate::memory::Memory,
---
> 	pub reload: crate::reload::Reload,
71c71
< 			memory: crate::memory::Memory::default(),
---
> 			reload: crate::reload::Reload::default(),
115,118d114
< 	/// Fire-and-forget emits run through this queue so a listener sees the
< 	/// events of one event name in emit order; a fresh task per emit races on
< 	/// the multithread runtime and reorders consecutive emits.
< 	pub(crate) queue: mpsc::UnboundedSender<(Value, Uid)>,
126c122
< 	pub(crate) memory: crate::memory::Memory,
---
> 	pub(crate) reload: crate::reload::Reload,
154c150
< 			memory: component.memory,
---
> 			reload: component.reload,
391,400d386
< 	pub fn memory(&self) -> Result<crate::memory::Memory, Error> {
< 		self
< 			.rt
< 			.reg
< 			.lock()
< 			.fibers
< 			.get(&self.fiber)
< 			.map(|f| f.memory.clone())
< 			.ok_or(Error::Gone)
< 	}
481c467
< 			crate::service::shared(value, f.memory.clone())
---
> 			crate::service::shared(value, f.reload.clone())
583c569
< 			let memory = fiber.memory.clone();
---
> 			let reload = fiber.reload.clone();
587c573
< 				let gate = memory.gate();
---
> 				let gate = reload.gate();
603,615d588
< 		let rt = self.rt.clone();
< 		let (queue, mut turn) = mpsc::unbounded_channel::<(Value, Uid)>();
< 		let deliver = f.clone();
< 		tokio::spawn(async move {
< 			// One consumer per registration: the listener runs to completion
< 			// before the next queued event, so emits arrive in order while
< 			// other listeners and blocked calls stay concurrent.
< 			while let Some((payload, emitter)) = turn.recv().await {
< 				if let Err(e) = deliver(payload).await {
< 					rt.fail(emitter, e);
< 				}
< 			}
< 		});
625d597
< 				queue,
662,688c634,642
< 		let queues = {
< 			let reg = self.rt.reg.lock();
< 			reg
< 				.listeners
< 				.get(name)
< 				.map(|l| {
< 					l.iter()
< 						.filter(|r| reg.fibers.get(&r.uid).is_some_and(|f| !f.staged))
< 						.map(|r| (r.queue.clone(), r.f.clone()))
< 						.collect::<Vec<_>>()
< 				})
< 				.unwrap_or_default()
< 		};
< 		for (queue, f) in queues {
< 			// A queue closed mid-emit means the registration was disposed after
< 			// the snapshot: deliver that last one directly, as the old
< 			// task-per-emit did.
< 			if queue.send((payload.clone(), self.fiber)).is_err() {
< 				let rt = self.rt.clone();
< 				let fiber = self.fiber;
< 				let payload = payload.clone();
< 				tokio::spawn(async move {
< 					if let Err(e) = f(payload).await {
< 						rt.fail(fiber, e);
< 					}
< 				});
< 			}
---
> 		for f in self.listeners(name) {
> 			let rt = self.rt.clone();
> 			let fiber = self.fiber;
> 			let payload = payload.clone();
> 			tokio::spawn(async move {
> 				if let Err(e) = f(payload).await {
> 					rt.fail(fiber, e);
> 				}
> 			});

## Report

spec01: exit 0
hi
exit=0  (0 healthy, 142 = hung in dyld)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
manifest untouched since the freeze

spec02: exit 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
bank is gone
fiber untouched since the freeze
runtime untouched since the freeze
720:			old.reload.finish();
821:		if !reload.begin() {
849:					reload.finish();
854:		let _calls = reload.gate().write_owned().await;
881:				reload.finish();
896:		reload.finish();

spec03: exit 0
hi
exit=0  (0 healthy, 142 = hung in dyld)
test tests::rpc_contract::rust_sdk_eof_rejects_another_inflight_call ... ok
test tests::process::sdk_child_roundtrip_errors_metadata_and_eof ... ok
test tests::socket::a_client_round_trips_through_a_cartridge ... ok
test turn::tests::a_credential_or_a_prompt_body_is_omitted_and_named ... ok
test turn::tests::the_sink_stays_bounded_by_rotating_one_generation ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test tests::process::ordinary_lua_composition_can_load_a_process_wrapper ... ok
test tests::folders::watcher_reloads_a_folder_when_its_manifest_changes ... ok
test tests::socket::socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors ... ok
{"msg":"rejected fixture migration","src":"peer","t":1789226349611,"turn":null}
test tests::process::lua_wrappers_merge_injections_before_start_and_preserve_config ... ok
test tests::rpc_contract::rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work ... ok
test tests::cartridges::wrapped_process_isolation_metadata_and_dependency_restart_compose ... ok
test tests::cartridges::local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges ... ok
test tests::process::watches_reload_wrappers_binaries_and_new_executable_directories_once ... ok

test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.94s

     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests zirkle

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

no bank in the suite
             8814592  maximum resident set size
median ms 6.269208010053262
