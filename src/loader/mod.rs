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
//!   "description": "Stores data", // optional: human-readable purpose
//!   "commands": {              // optional: documentation, never auto-executed
//!     "test": {"argv": ["just", "test", "store"], "cwd": "."}
//!   },
//!   "binary": "store-bin",      // optional: executable basename, when it
//!                               //   differs from the cartridge folder
//!   "ui": "ui/index.ts",        // optional: Solid UI module inside this folder
//!   "selftest": "store.check",  // optional: a provided key proving behaviour
//!   "integration": "store.wire",// optional: a provided key proving wiring
//!   "source": "https://…",      // optional: where to retrieve source from
//!   "settings": {              // optional: the config keys this cartridge
//!     "max_bytes": {           //   contributes; dotted names nest
//!       "type": "integer",     //   integer|number|boolean|string|list|table
//!       "default": 67108864,   //   required: the value when nothing sets it
//!       "min": 1024,           //   optional inclusive bounds, numbers only
//!       "doc": "Read budget."  //   optional: one line, printed by `settings`
//!     }
//!   },
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
//! **A declared setting is a filled setting.** Every key in `settings` holds a
//! value by the time the cartridge starts: its own default, or what
//! `~/.cartridge/config.lua`, the project's `.cartridge/config.lua` or the
//! profile entry laid over it, in that order. The value is checked against the
//! declared kind and bounds first, so a cartridge never reads a key that is
//! missing, of the wrong type, or outside what it said it could take — and
//! never needs a fallback beside the declaration to say so twice.
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
//!
//! The module is laid out as the document is used:
//!
//! - [`document`] — the format: `cartridge.json` read, checked, resolved;
//! - [`entries`] — the profile: the ledger's entries, `init.lua` over them,
//!   `config.lua` over that, and each entry loaded as a component;
//! - [`foreground`] — one run of the profile: a service called, contracts
//!   verified, everything disposed;
//! - [`reconcile`] — the long-lived host: entries added, removed and replaced
//!   in place, the generation switch that keeps services stable;
//! - [`watch`] — the sources observed, so a change is a replacement;
//! - [`bridge`] — a client module's calls into its own cartridge.

mod bridge;
mod document;
mod entries;
mod foreground;
mod reconcile;
mod watch;

use std::path::{Path, PathBuf};

use crate::runtime::FiberHandle;

use document::classify;
pub(crate) use document::{document, resolve, Declared};
pub use document::{Cartridge, Command, Grant, MANIFEST};

/// A profile entry names a cartridge folder; its manifest selects the Lua entry.
/// Explicit Lua files remain supported for programmatic composition.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
	pub id: String,
	pub path: String,
	#[serde(default)]
	pub config: serde_json::Value,
	#[serde(default)]
	pub disabled: bool,
	#[serde(default)]
	pub isolate: Vec<String>,
	#[serde(default)]
	pub inject: Vec<String>,
}

impl Entry {
	pub fn file(&self, dir: &Path) -> PathBuf {
		normalize(&classify(&dir.join(&self.path)))
	}
}

/// One entry of the profile as the files describe it, before any fiber runs.
pub struct CartridgeInfo {
	pub entry: Entry,
	pub inject: Vec<String>,
	pub provide: Vec<String>,
	/// Inner keys this cartridge passes outward, straight off its document.
	pub export: Vec<String>,
	/// The capability request, straight off the same document. `None` when the
	/// document could not be read at all — absent, never empty, because an empty
	/// grant is the tightest policy and must not double as "could not be read".
	pub grant: Option<Grant>,
	/// Why the document would not read, when it would not. Kept apart from
	/// `error` because they are different facts: this one is about the document
	/// and is known whether or not anything tried to load the cartridge, while
	/// `error` is what happened when the entry was evaluated. Conflating them is
	/// the same mistake as keeping the request in a second file.
	pub unread: Option<String>,
	pub error: Option<String>,
}

/// One entry's configurable surface: what it declares, what the layers settled
/// it to, and what it is configured with that it never declared.
pub struct SettingsInfo {
	pub id: String,
	pub path: String,
	pub disabled: bool,
	/// The keys this cartridge contributes, dotted and sorted.
	pub specs: crate::settings::Specs,
	/// Those keys filled in, with every layer laid over them.
	pub settled: serde_json::Value,
	/// Configured keys no declaration mentions: the migration checklist.
	pub undeclared: Vec<String>,
}

pub(crate) struct Loaded {
	pub(crate) entry: Entry,
	pub(crate) fiber: Option<FiberHandle>,
	pub(crate) sources: Vec<Source>,
	pub(crate) error: Option<String>,
	pub(super) reload: crate::reload::Reload,
}

#[derive(Clone)]
pub(crate) struct Source {
	pub(crate) path: PathBuf,
	pub(crate) stamp: Option<(u64, std::time::SystemTime)>,
}

impl Source {
	pub(super) fn new(path: PathBuf) -> Self {
		let stamp = std::fs::metadata(&path)
			.ok()
			.and_then(|m| Some((m.len(), m.modified().ok()?)));
		Self { path, stamp }
	}

	pub(crate) fn changed(&self) -> bool {
		self.stamp != Self::new(self.path.clone()).stamp
	}
}

pub(super) fn normalize(path: &Path) -> PathBuf {
	path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The one user profile: the `.cartridge` directory of the project being run,
/// holding the `init.lua` that composes it and the `config.lua` that configures
/// it. There is no name to select and no `<profile>` layer beneath it — a
/// command picks an entry point out of this one host, never a second list of
/// cartridges — so every caller resolves the same directory.
pub fn profile() -> PathBuf {
	PathBuf::from(".cartridge")
}

/// Cartridges are loaded from the working directory unless `--dir` is supplied.
pub fn builtin() -> PathBuf {
	PathBuf::from("builtin")
}

/// The project a runtime belongs to: the nearest directory at or above the one
/// it was started in whose `.cartridge` folder composes a runtime.
///
/// The composing `init.lua` is what makes a directory a project, not the
/// `.cartridge` folder alone — a repository keeps memos, records and data under
/// that name without ever being a project, and stopping at the first of those
/// would bind the runtime to a directory that has no composition to load.
///
/// A runtime is bound to its project. Two directories run two of them, each
/// with its own cartridges, its own data directories and its own credential
/// store, and each dying with the terminal that started it — so the way to move
/// a runtime is to end it here and start it there. That only holds if the
/// project is decided by where the runtime was started and nothing else: a
/// profile can be an absolute path shared by every project on the machine, and
/// reading the project off the profile would collapse them all into one.
///
/// Searching upward is what lets a command typed deep inside a project reach
/// the runtime serving it instead of starting a second one in a subdirectory.
/// A working directory under no project at all is its own root: the runtime
/// still starts, and its relative paths land where it was started.
pub fn root() -> PathBuf {
	let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	cwd.ancestors()
		.find(|dir| dir.join(".cartridge").join("init.lua").is_file())
		.map_or(cwd.clone(), Path::to_path_buf)
}
