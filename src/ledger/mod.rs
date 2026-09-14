//! The ledger: every cartridge installed under one root, found on the
//! filesystem. A directory holding a `cartridge.json` is a cartridge, at any
//! depth below another; an entry's identity is its path from the root. A
//! document that will not read is still an entry, with `unread` saying why.

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
	/// cartridge's `listen` is seen by every lookup made from inside its
	/// parent's subtree — the walk passes the parent's scope — and by nothing
	/// outside the parent unless a parent passes it on: that is the settled
	/// reading of "inner cartridges are hidden until passed on", and
	/// the-manifest's "satisfies its parent's needs and nothing else" names
	/// the graph outside the parent, not the siblings within it.
	/// Events this cartridge listens to: what a `needs` of another resolves to.
	pub listen: Vec<String>,
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

	pub fn offers(&self) -> impl Iterator<Item = &String> {
		self.listen.iter()
	}
}

/// What a lookup found, and where the walk stopped.
pub enum Bound<'a> {
	/// Exactly one entry of the nearest offering scope listens to it.
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

	/// Who listens to `key`, apart from `from` itself: one entry, none, or a clash.
	pub fn resolve(&self, from: &str, key: &str) -> Bound<'_> {
		let offered: Vec<&Installed> = self
			.entries()
			.filter(|e| e.path != from && e.offers().any(|k| k == key))
			.collect();
		match offered.len() {
			0 => Bound::None,
			1 => Bound::One(offered[0]),
			_ => Bound::Clashed(offered),
		}
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

/// The document as data: declarations are recorded even when the Lua entry is missing.
fn read_entry(folder: &Path, path: String) -> Installed {
	let manifest = folder.join(MANIFEST);
	match Cartridge::document(&manifest) {
		Ok(doc) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: doc.name,
			listen: doc.listen,
			needs: doc.needs,
			unread: None,
		},
		Err(e) => Installed {
			path,
			dir: folder.to_path_buf(),
			name: String::new(),
			listen: Vec::new(),
			needs: Vec::new(),
			unread: Some(e.to_string()),
		},
	}
}
