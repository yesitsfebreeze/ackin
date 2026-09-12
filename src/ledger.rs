//! The ledger: every cartridge installed under one root, derived from the
//! filesystem rather than from a hand-kept list.
//!
//! **Installing is putting a tree where the ledger looks; uninstalling is
//! taking it away.** Nothing between the two edits a list, because a list that
//! must be edited is a second place for the truth to live.
//!
//! The ledger is a **namespace of subtrees, not a flat table.** An entry is
//! identified by its [`Installed::path`] from the root — never by its bare
//! name — so two cartridges may provide the same key without colliding. What a
//! cartridge can see is its own subtree first, then its parent's, outward to
//! the root; a key deeper than one level reaches it only where every parent
//! between re-exported it. That rule is [`crate::loader`]'s, settled with the
//! document format; this module is the same rule applied to a whole tree
//! instead of to one cartridge's children.
//!
//! **What is scanned.** A directory is a cartridge exactly when it holds a
//! [`crate::loader::MANIFEST`]. The root's direct children are the top-level
//! entries; a cartridge's direct children are its nested entries, one level at
//! a time, forever. A directory that is not a cartridge is not descended into,
//! so `outer/vendor/inner` is in no subtree at all when `outer/vendor` holds no
//! document — the subtree a key may travel through is a chain of cartridges,
//! and a plain folder does not extend it.
//!
//! **An unreadable document is still an entry.** Its `unread` says why and its
//! declarations are empty. The read is the host's read: the document's own
//! checks *and* the re-export check against the subtree, so the ledger refuses
//! — as `unread` — exactly the trees the host refuses to load, and the two
//! listings can never disagree about what reads. Absent is not the same fact
//! as empty, and a document that would not read must not read as one that
//! offered nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::loader::{Cartridge, MANIFEST};

/// One cartridge as the ledger found it.
pub struct Installed {
	/// The identity: the `/`-joined path from the ledger root. `outer/inner`
	/// for a cartridge nested in `outer`. Never the bare name, which is not
	/// unique across the tree and was never meant to be.
	pub path: String,
	/// Where it sits on disk.
	pub dir: PathBuf,
	/// The document's own `name`. Empty when the document would not read.
	pub name: String,
	/// Keys offered, private to this cartridge's own subtree. A nested
	/// cartridge's `provide` is seen by every lookup made from inside its
	/// parent's subtree — the walk passes the parent's scope — and by nothing
	/// outside the parent unless a parent passes it on: that is the settled
	/// reading of "inner cartridges are hidden until passed on", and
	/// the-manifest's "satisfies its parent's needs and nothing else" names
	/// the graph outside the parent, not the siblings within it.
	pub provide: Vec<String>,
	/// Inner keys passed outward under this cartridge's name.
	pub export: Vec<String>,
	/// Keys asked for, resolved outward from here by [`Ledger::resolve`].
	pub needs: Vec<String>,
	/// Why the document would not read, when it would not.
	pub unread: Option<String>,
}

impl Installed {
	/// The path of the cartridge this one is nested in, or `None` at the top.
	pub fn parent(&self) -> Option<&str> {
		self.path.rsplit_once('/').map(|(head, _)| head)
	}

	/// What this entry makes visible to whatever contains it: its own keys plus
	/// the inner ones it passes on. `export` is validated on the ledger's read
	/// the same way the host validates it — [`Cartridge::passed_on`], against the
	/// subtree at document-read time — so a re-export here names something real,
	/// and a document the host refuses to load carries `unread` here too.
	pub fn offers(&self) -> impl Iterator<Item = &String> {
		self.provide.iter().chain(self.export.iter())
	}
}

/// What a lookup found, and where the walk stopped.
pub enum Bound<'a> {
	/// Exactly one entry of the nearest offering scope provides it.
	One(&'a Installed),
	/// Two or more entries of *one* scope offer the key, in path order. The
	/// ask has no answer until the tree names them differently.
	Clashed(Vec<&'a Installed>),
	/// Nothing in any scope the walk passes through offers it.
	None,
}

impl Bound<'_> {
	/// The paths the binding named, empty for [`Bound::None`].
	pub fn paths(&self) -> Vec<&str> {
		match self {
			Bound::One(e) => vec![e.path.as_str()],
			Bound::Clashed(offered) => offered.iter().map(|e| e.path.as_str()).collect(),
			Bound::None => Vec::new(),
		}
	}

	/// True when the walk found offers enough to be unable to pick one.
	pub fn is_clashed(&self) -> bool {
		matches!(self, Bound::Clashed(_))
	}
}

/// Every cartridge under one root, keyed by its path from that root.
pub struct Ledger {
	entries: BTreeMap<String, Installed>,
}

impl Ledger {
	/// Read the tree. A root that does not exist is an empty ledger and not an
	/// error: nothing installed is a state the host runs in, and the scan says
	/// so by holding nothing rather than by failing.
	pub fn scan(root: &Path) -> Self {
		let mut entries = BTreeMap::new();
		descend(root, "", &mut entries);
		Self { entries }
	}

	/// Every entry, in path order — so the listing does not depend on the order
	/// the filesystem happened to hand the directories back.
	pub fn entries(&self) -> impl Iterator<Item = &Installed> {
		self.entries.values()
	}

	pub fn get(&self, path: &str) -> Option<&Installed> {
		self.entries.get(path)
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	/// The entries directly inside `scope` — `""` for the root's own children.
	/// This is the unit of visibility: a key is a candidate for a lookup in
	/// `scope` exactly when one of these entries offers it.
	pub fn children(&self, scope: &str) -> Vec<&Installed> {
		self.entries
			.values()
			.filter(|e| e.parent().unwrap_or("") == scope)
			.collect()
	}

	/// The scopes a lookup from `from` passes through, nearest first: the asking
	/// cartridge's own subtree, then its parent's, outward to the root.
	fn outward(from: &str) -> Vec<&str> {
		let mut scopes = vec![from];
		let mut at = from;
		while let Some((head, _)) = at.rsplit_once('/') {
			scopes.push(head);
			at = head;
		}
		if !from.is_empty() {
			scopes.push("");
		}
		scopes
	}

	/// Bind `key` for the cartridge at `from`. **A walk, not a map hit.** The
	/// asking cartridge's own subtree answers first; only where it is silent
	/// does the search step outward, one containing subtree at a time. A key one
	/// subtree over is invisible however identical its name, because it was never
	/// offered into any scope this walk passes through. The one visible side of
	/// that privacy: a nested cartridge's `provide` is a candidate for every
	/// lookup whose walk passes its parent's scope, so everything inside the
	/// parent's subtree — the parent, the other children, their descendants —
	/// sees it, and nothing outside the parent does until a parent passes it on.
	///
	/// The settled rule — two cartridges may provide the same key without
	/// colliding — holds across scopes and does not hold inside one. Where two
	/// entries of the *same* scope offer the key the walk stops and says so
	/// ([`Bound::Clashed`]) rather than quietly taking the first in path order:
	/// a clash you find when you install is a diagnostic, a clash you find from
	/// odd behaviour later is a bug.
	///
	/// `from` is `""` for a lookup made at the root itself.
	pub fn resolve(&self, from: &str, key: &str) -> Bound<'_> {
		for scope in Self::outward(from) {
			// Path order, so a clash names the same offers on every run rather
			// than by directory order. Sorting decides what a clash *names*; it
			// does not decide a winner.
			let offered: Vec<&Installed> = self
				.children(scope)
				.into_iter()
				.filter(|e| e.path != from && e.offers().any(|k| k == key))
				.collect();
			match offered.len() {
				0 => continue,
				1 => return Bound::One(offered[0]),
				_ => return Bound::Clashed(offered),
			}
		}
		Bound::None
	}

	/// Every need of every entry, paired with what it binds to — [`Bound::None`]
	/// where nothing in scope offers it, [`Bound::Clashed`] where a scope offers
	/// it twice. The registry the resolver reads, in the one shape a report and
	/// a launch both want.
	pub fn bindings(&self) -> Vec<(&Installed, &String, Bound<'_>)> {
		self.entries()
			.flat_map(|e| {
				e.needs
					.iter()
					.map(move |key| (e, key, self.resolve(&e.path, key)))
			})
			.collect()
	}
}

/// One level, then the same again inside each cartridge found. A directory
/// holding no document is not descended into: it is not a cartridge, so it
/// cannot pass a key on, and a subtree it hid would be reachable by nobody.
fn descend(dir: &Path, scope: &str, into: &mut BTreeMap<String, Installed>) {
	let Ok(read) = std::fs::read_dir(dir) else {
		return;
	};
	let mut folders: Vec<PathBuf> = read
		.flatten()
		.map(|e| e.path())
		.filter(|p| p.join(MANIFEST).is_file())
		.collect();
	folders.sort();
	for folder in folders {
		let Some(name) = folder.file_name().and_then(|n| n.to_str()) else {
			continue;
		};
		let path = if scope.is_empty() {
			name.to_owned()
		} else {
			format!("{scope}/{name}")
		};
		into.insert(path.clone(), read_entry(&folder, path.clone()));
		descend(&folder, &path, into);
	}
}

/// The document as data. `Cartridge::document` is used rather than
/// `Cartridge::read`, on the same grounds the manifest listing uses it: a
/// cartridge whose Lua entry is missing has still declared what it declares,
/// and the ledger records declarations, not evaluations. On top of the
/// document's own checks the read runs [`Cartridge::passed_on`] — the same
/// re-export validation the host's reader runs — so `unread` here and the
/// host's refusal are the same verdict on the same tree.
fn read_entry(folder: &Path, path: String) -> Installed {
	let manifest = folder.join(MANIFEST);
	let read = Cartridge::document(&manifest).and_then(|doc| {
		doc.passed_on(folder, &manifest)?;
		Ok(doc)
	});
	match read {
		Ok(doc) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: doc.name,
			provide: doc.provide,
			export: doc.export,
			needs: doc.needs,
			unread: None,
		},
		Err(e) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: String::new(),
			provide: Vec::new(),
			export: Vec::new(),
			needs: Vec::new(),
			unread: Some(e),
		},
	}
}
