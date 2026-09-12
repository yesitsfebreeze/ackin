---
state: done
origin: requested
priority: 85
complexity: 33
blast-radius: mid
needs:
  - the-host-binary
workflow: probe-then-spec
actual: 0.64h
---

# The manifest

What one cartridge declares about itself, and nothing beyond it: its name, what
it provides, what it needs, how it is entered. Today that is `cartridge.json`
plus a Lua entry returning `zirkle.process(...)`, described in `~/dev/sys/CLAUDE.md`
and implemented in `src/cartridge.rs` and `src/loader.rs`.

The change this PRD makes is that the same document is also the capability
request. What a cartridge declares it needs — filesystem paths, network, the
right to exec — is both what the resolver grants it and what the sandbox
confines it to. One document, read twice, by [[the-resolver]] and by
[[the-sandbox]]. There is no second grant file and no separate policy.

This is what makes uninstalling mean something: what a cartridge takes away when
it goes is exactly what it declared on the way in.

## Nesting is real, and a name travels only when it is passed on

The fork below is **answered**, and it is a premise of this PRD rather than a
question inside it. The user chose *hidden until passed on*: each piece names
what it offers outward, so names never clash and nesting is real.

Read as a contract on the document:

- A cartridge's `provide` keys are **private to that cartridge's own subtree by
  default.** A nested cartridge satisfies its parent's needs and nothing else;
  the graph outside the parent cannot see it and cannot bind to it.
- A parent that wants an inner name visible outward **says so explicitly** in
  its own manifest. That re-export is a declaration like any other — it is in
  the same document, and it is subject to the same reading by
  [[the-resolver]] and [[the-sandbox]].
- Therefore **two cartridges may provide the same key** without colliding, so
  long as neither subtree re-exports it into the other. A name is unique
  within a subtree, never across the whole graph.

This decides the shape of the data, which is why it was settled first. The
[[the-ledger]] is a **namespace of subtrees**, not a flat table: a key resolves
against the asking cartridge's subtree and walks outward, and an entry is
identified by its path, not by its bare name. [[the-resolver]]'s lookup walks
outward through subtrees rather than hitting a single map, and uninstalling a parent takes its
whole subtree with it — which is the same property that makes uninstalling mean
something, stated one level down.

The cost the user accepted is the wiring: an inner capability that genuinely
belongs to the outside has to be named twice, once where it is provided and
once where it is passed on. The cost they refused is a single global namespace
where the second cartridge to claim a name either loses or wins by accident.

## Questions

### Q1: What an inner cartridge offers the outside *(answered — see above)*

When one capability is built out of others, you are choosing whether the things
those inner pieces offer are visible to everything outside, or hidden until the
outer one chooses to pass them on. Visible everywhere means fewer steps and more
name clashes; hidden means real nesting and more wiring?

1. **Hidden until passed on** — each piece names what it offers outward, so names never clash and nesting is real. (recommended)
2. **Visible everywhere** — whatever an inner piece offers, the outer one offers too, with no wiring and possible name clashes.
3. **Visible unless hidden** — everything passes outward by default, and a piece can name the few things it keeps to itself.

<!-- for the board: cartridge.json `provide` keys; src/loader.rs inject-to-provider search and src/main.rs deps(). Flat vs subtree namespace decides the ledger's data structure and the resolver's lookup. Answer lands in the-manifest spec01, and is a premise for the-ledger and the-resolver. -->

## Answers

**Q1** *(answered 2026-09-12 15:39)* — Hidden until passed on — each piece names what it offers outward, so names never clash and nesting is real.

## Report

spec01: exit 0

running 15 tests
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

32://! not the loosest. Every field added after `source` is `#[serde(default)]`,
74:#[serde(deny_unknown_fields)]
78:	#[serde(default)]
80:	#[serde(default)]
82:	#[serde(default)]
84:	#[serde(default)]
147:#[serde(deny_unknown_fields)]
166:	#[serde(default)]
170:	#[serde(default)]
174:	#[serde(default)]
179:	#[serde(default)]
186:#[serde(deny_unknown_fields)]
190:	#[serde(default)]
193:	#[serde(default)]
196:	#[serde(default)]
199:	#[serde(default)]
src/lua.rs:217:		let declared = crate::loader::resolve(path)?;
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.35s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
    Checking zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.05s

spec02: exit 0

running 15 tests
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

389:	pub fn offered(root: &Path) -> Result<Vec<String>, String> {
403:	fn passed_on(&self, root: &Path, manifest: &Path) -> Result<(), String> {
524:	let (manifest, entry) = Cartridge::read(&path).map_err(mlua::Error::RuntimeError)?;
825:			// `Cartridge::read` and not `resolve`: this walks the profile for the
830:			let (cartridge, _) = Cartridge::read(&manifest).map_err(mlua::Error::RuntimeError)?;
--- list ---
outer  outer  exports inner.store  writes cache  execs rg
  inner.store <- other
other  other  provides inner.store
--- the same document with a re-export nothing offers ---
lone  lone  error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.PPA4GkKKrE/lone/cartridge.json: `nobody.key` is passed on, but nothing inside this cartridge offers it
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.37s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)

spec03: exit 0
--- list ---
outer  outer  exports inner.store  writes cache  execs rg
  inner.store <- other
other  other  provides inner.store
--- the same document with a re-export nothing offers ---
lone  lone  error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.zL14EHw5hf/lone/cartridge.json: `nobody.key` is passed on, but nothing inside this cartridge offers it
--- 15 documents copied; list exits 0 only if every document reads ---
agent  agent
fs  fs
gitfs  gitfs
harness  harness
mcp  mcp
memo  memo
memory-tool  memory-tool
memory  memory
policy  policy
proxy  proxy
pty  pty
router  router
sessions  sessions
tools  tools
ui  ui  error: runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.mlw8EjW0yz/ui/ui/index.ts: No such file or directory (os error 2)
227:fn list(cartridges: &[CartridgeInfo]) -> usize {
228-	for p in cartridges {
229-		let mut line = format!("{}  {}", p.entry.id, p.entry.path);
230-		if p.entry.disabled {
231-			line.push_str("  (disabled)");
232-		}
233-		if !p.provide.is_empty() {
234-			line.push_str(&format!("  provides {}", p.provide.join(", ")));
235-		}
236-		if !p.export.is_empty() {
237-			line.push_str(&format!("  exports {}", p.export.join(", ")));
238-		}
239-		// What it asked for is what it gets and the wall it hits, so it is listed
240-		// next to what it provides rather than in a second place.
241-		if let Some(grant) = &p.grant {
242-			for (label, asked) in [
243-				("reads", &grant.read),
244-				("writes", &grant.write),
245-				("net", &grant.net),
246-				("execs", &grant.exec),
247-			] {
248-				if !asked.is_empty() {
249-					line.push_str(&format!("  {label} {}", asked.join(", ")));
250-				}
251-			}
252-		}
253-		// At most one of the two is ever set: `unread` is the document's own
254-		// failure and `error` is what evaluating the entry hit, and `manifest`
255-		// does not report the first twice.
256-		for note in [p.unread.as_deref(), p.error.as_deref()].into_iter().flatten() {
257-			line.push_str(&format!("  error: {note}"));
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s

spec04: exit 0
//! The cartridge document, and the one place its format is written.
//!
//! A cartridge declares itself in `cartridge.json`, and that same document is
//! its capability request. It is read as data: parsing it never evaluates the
//! Lua entry, so what a cartridge asks for is readable before anything of it
//! runs, and a disabled cartridge still declares what it would ask for.
//!
//! ```json
//! {
//!   "name": "store",            // required: the cartridge's own name
//!   "entry": "init.lua",        // required: Lua entry, relative to this folder
//!   "binary": "store-bin",      // optional: executable basename, when it
//!                               //   differs from the cartridge folder
//!   "ui": "ui/index.ts",        // optional: Solid UI module inside this folder
//!   "selftest": "store.check",  // optional: a provided key proving behaviour
//!   "integration": "store.wire",// optional: a provided key proving wiring
//!   "source": "https://…",      // optional: where to retrieve source from
//!   "provide": ["store.get"],   // optional: keys offered, private by default
//!   "needs": ["log.write"],     // optional: keys asked for
//!   "export": ["inner.store"],  // optional: inner keys passed outward
//!   "grant": {                  // optional: the capability request
//!     "read":  ["data"],        //   paths readable, relative or absolute
//!     "write": ["cache"],       //   paths writable; writable implies readable
//!     "net":   ["api.host"],    //   hosts reachable, or the bare `*`
//!     "exec":  ["rg"]           //   programs runnable, by basename or path
//!   }
//! }
//! ```
//!
//! **An absent `grant` grants nothing.** `Grant::default()` is empty on all
//! four of `read`, `write`, `net` and `exec`, which is the tightest policy and
//! not the loosest. Every field added after `source` is `#[serde(default)]`,
//! so a document written before the capability request existed still reads,
//! while `deny_unknown_fields` keeps a misspelled one an error.
//!
//! **The document wins wherever it speaks.** Where `provide` or `needs` is
//! non-empty it replaces what the Lua entry declares; where the document is
//! silent — or where a bare `.lua` path is entered, which has no document at
//! all — the entry stays the only source.
//!
//! **A name travels outward only where a parent passes it on.** A cartridge's
//! `provide` keys are private to its own subtree: a nested cartridge satisfies
//! its parent and nothing else. A parent makes an inner key visible outward by
//! naming it in its own `export`, one level at a time, so a deep key reaches
//! the top only when every level between re-exports it. Two cartridges may
//! therefore provide the same key without colliding. A key is unique within a
//! subtree, never across the graph.
//!
//! **Two readers, one document.** The resolver takes `provide`, `needs` and
//! `export` and binds them; the sandbox takes `grant` and confines to it.
//! There is no second grant file and no separate policy document — what a
//! cartridge is granted is what it declared on the way in, which is what makes
//! uninstalling it mean something.
//!
//! **What this module does not settle.** Nothing here loads a nested
//! cartridge: `Cartridge::offered` reads a subtree off the filesystem to
//! validate a re-export, and that is all. Where the ledger looks for
//! cartridges, and how a need is resolved outward through subtrees, are the
//! ledger's and the resolver's contracts, not this format's.

name=1 entry=1 binary=1 ui=1 selftest=1 integration=1 source=1 provide=1 needs=1 export=1 grant=1 read=1 write=1 net=1 exec=1 
 Documenting zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.33s
   Generated /Users/feb/dev/cartridge/.pearde/.lanes/the-manifest/target/doc/zirkle/index.html
