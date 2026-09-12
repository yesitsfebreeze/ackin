//! The resolver: the ledger's walk as the launch path.
//!
//! [`crate::main`]'s `deps` walks a cartridge's injections for display; this
//! module is the same walk pointed at a launch. Starting a tool resolves the
//! chain its provider stands on, **to its far end**, and the launch starts
//! from there: the far end's program comes up and re-enters the binary for
//! the next link, one re-entry per link, until the named tool is running at
//! the top of a tree that was resolved entirely at runtime, out of whatever
//! the ledger held at the moment of the ask.
//!
//! Three things refuse a chain, and each refusal is a diagnostic naming what
//! it read, never a silent pick:
//!
//! - a key nothing in scope offers — the ledger says `?`; a need nothing
//!   offers is legal to list, but the launch cares, not the registry, and
//!   uninstalling is the absence of a launch;
//! - a key one scope offers twice — the ledger's [`crate::ledger::Bound::Clashed`]
//!   is carried through, and the ask fails naming every offer;
//! - a chain that closes on itself — the same stack `deps` tracks, inherited
//!   as a duty: the launch path refuses the cycle the listing would name.

use crate::ledger::{Bound, Installed, Ledger};
use std::collections::HashSet;

/// Why the chain could not be resolved.
#[derive(Debug, PartialEq)]
pub enum Refusal {
	/// The key binds to nothing in any scope the walk passes through.
	Unbound { from: String, key: String },
	/// One scope offers the key more than once; the ask names every offer.
	Ambiguous { key: String, offered: Vec<String> },
	/// The chain closes on a node it is already walking through.
	Cycle { key: String, at: String },
}

impl std::fmt::Display for Refusal {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Unbound { from, key } => {
				write!(f, "`{key}` asked for at `{from}` binds to nothing in scope")
			}
			Self::Ambiguous { key, offered } => write!(
				f,
				"`{key}` is ambiguous: {} all offer it",
				offered.join(", ")
			),
			Self::Cycle { key, at } => write!(f, "`{key}` resolves into `{at}` again: cycle"),
		}
	}
}

/// The chain starting `key` from `from` requires, **bottom-up**: the far end
/// first, the entry that provides `key` last. `from` is `""` for an ask made
/// at the root — the asker of a tool is whoever is asking, not a cartridge.
pub fn chain<'a>(ledger: &'a Ledger, from: &str, key: &str) -> Result<Vec<&'a Installed>, Refusal> {
	let mut chain = Vec::new();
	walk(
		ledger,
		from,
		key,
		&mut chain,
		&mut Vec::new(),
		&mut HashSet::new(),
	)?;
	Ok(chain)
}

/// One lookup, then the provider's own needs, deepest first. The stack is the
/// walk's cycle duty: a provider already being walked is a closed chain, and
/// the walk stops there rather than launching it twice. A diamond is not a
/// cycle — two nodes may ask for the same provider and both get it, which is
/// why the provider is pushed on the way down and popped on the way up, and
/// why the chain deduplicates by path.
fn walk<'a>(
	ledger: &'a Ledger,
	from: &str,
	key: &str,
	chain: &mut Vec<&'a Installed>,
	stack: &mut Vec<String>,
	completed: &mut HashSet<String>,
) -> Result<(), Refusal> {
	match ledger.resolve(from, key) {
		Bound::None => Err(Refusal::Unbound {
			from: from.to_owned(),
			key: key.to_owned(),
		}),
		Bound::Clashed(offered) => Err(Refusal::Ambiguous {
			key: key.to_owned(),
			offered: offered.iter().map(|e| e.path.clone()).collect(),
		}),
		Bound::One(provider) => {
			if stack.iter().any(|path| path == &provider.path) {
				return Err(Refusal::Cycle {
					key: key.to_owned(),
					at: provider.path.clone(),
				});
			}
			if completed.contains(&provider.path) {
				return Ok(());
			}
			stack.push(provider.path.clone());
			for need in &provider.needs {
				walk(ledger, &provider.path, need, chain, stack, completed)?;
			}
			stack.pop();
			completed.insert(provider.path.clone());
			chain.push(provider);
			Ok(())
		}
	}
}
