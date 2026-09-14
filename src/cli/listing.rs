//! `list` and `ledger`: what is composed and what is installed, one line per
//! cartridge with each of its needs resolved to a provider under it.

use std::process::ExitCode;

use cartridge::host::Host;
use cartridge::loader::CartridgeInfo;
use cartridge::{Error, Result};

use super::{Project, FAILED};

/// A count of problems as an exit code: zero is success, anything else says
/// the listing found something to fix.
fn exit(problems: usize) -> ExitCode {
	match problems {
		0 => ExitCode::SUCCESS,
		_ => ExitCode::from(FAILED),
	}
}

pub(crate) fn list(project: &Project) -> Result<ExitCode> {
	let host = Host::new(&project.dir, &project.profile, false)?;
	let cartridges = host.manifest().map_err(|e| {
		Error::Profile(format!(
			"{}: {e}",
			project.profile.join("init.lua").display()
		))
	})?;
	// A document that would not read is a failure of the listing, not a
	// footnote in it: the line is printed, and the exit says so.
	Ok(exit(lines(&cartridges)))
}

pub(crate) fn ledger(project: &Project) -> ExitCode {
	let ledger = cartridge::ledger::Ledger::scan(&project.dir);
	exit(ledger_lines(&ledger))
}

fn deps(all: &[CartridgeInfo], cartridge: &CartridgeInfo, depth: usize, stack: &mut Vec<String>) {
	let pad = "  ".repeat(depth);
	for key in &cartridge.needs {
		let provider = all
			.iter()
			.find(|p| !p.entry.disabled && p.provide.iter().any(|k| k == key));
		match provider {
			None => println!("{pad}{key} <- ?"),
			Some(p) if stack.contains(&p.entry.id) => {
				println!("{pad}{key} <- {} (cycle)", p.entry.id)
			}
			Some(p) => {
				println!("{pad}{key} <- {}", p.entry.id);
				stack.push(p.entry.id.clone());
				deps(all, p, depth + 1, stack);
				stack.pop();
			}
		}
	}
}

/// Prints one line per cartridge and answers how many **documents** could not be
/// read — not how many entries failed, which is a different and larger number:
/// a cartridge whose Lua entry or declared `ui` file is missing has still made
/// its declarations, and they are printed. An unreadable document prints why
/// and no declaration columns at all, so it never reads as a document that
/// asked for nothing.
fn lines(cartridges: &[CartridgeInfo]) -> usize {
	for p in cartridges {
		let mut line = format!("{}  {}", p.entry.id, p.entry.path);
		if p.entry.disabled {
			line.push_str("  (disabled)");
		}
		if !p.provide.is_empty() {
			line.push_str(&format!("  provides {}", p.provide.join(", ")));
		}
		if !p.on.is_empty() {
			line.push_str(&format!("  on {}", p.on.join(", ")));
		}
		// What it asked for is what it gets and the wall it hits, so it is listed
		// next to what it provides rather than in a second place.
		if let Some(grant) = &p.grant {
			for (label, asked) in [
				("reads", &grant.read),
				("writes", &grant.write),
				("net", &grant.net),
				("execs", &grant.exec),
			] {
				if !asked.is_empty() {
					line.push_str(&format!("  {label} {}", asked.join(", ")));
				}
			}
		}
		// At most one of the two is ever set: `unread` is the document's own
		// failure and `error` is what evaluating the entry hit, and `manifest`
		// does not report the first twice.
		for note in [p.unread.as_deref(), p.error.as_deref()]
			.into_iter()
			.flatten()
		{
			line.push_str(&format!("  error: {note}"));
		}
		println!("{line}");
		deps(cartridges, p, 1, &mut vec![p.entry.id.clone()]);
	}
	cartridges.iter().filter(|p| p.unread.is_some()).count()
}

/// One line per installed cartridge, in path order, with each of its needs
/// under it bound to the path that provides it — `?` where nothing in scope
/// does, `ambiguous` naming every offer where one scope offers it twice.
/// Answers how many **problems** the listing found, on the same terms as
/// [`lines`]: an unreadable document is listed and counted, never silently
/// absent and never printed as a cartridge that declared nothing, and a need
/// two entries of one scope offer is counted too, so a clash found at install
/// time is a non-zero exit rather than odd behaviour later.
fn ledger_lines(ledger: &cartridge::ledger::Ledger) -> usize {
	for e in ledger.entries() {
		let mut line = e.path.clone();
		if !e.name.is_empty() && e.name != e.path.rsplit('/').next().unwrap_or("") {
			line.push_str(&format!("  ({})", e.name));
		}
		if !e.provide.is_empty() {
			line.push_str(&format!("  provides {}", e.provide.join(", ")));
		}
		if let Some(why) = &e.unread {
			line.push_str(&format!("  error: {why}"));
		}
		println!("{line}");
		for key in &e.needs {
			match ledger.resolve(&e.path, key) {
				cartridge::ledger::Bound::One(p) => println!("  {key} <- {}", p.path),
				cartridge::ledger::Bound::None => println!("  {key} <- ?"),
				cartridge::ledger::Bound::Clashed(offered) => println!(
					"  {key} <- ambiguous ({})",
					offered
						.iter()
						.map(|p| p.path.as_str())
						.collect::<Vec<_>>()
						.join(", ")
				),
			}
		}
	}
	ledger.entries().filter(|e| e.unread.is_some()).count()
		+ ledger
			.bindings()
			.iter()
			.filter(|(_, _, bound)| bound.is_clashed())
			.count()
}
