use std::process::ExitCode;

use cartridge::host::Host;
use cartridge::loader::CartridgeInfo;
use cartridge::{Error, Result};

use super::{Project, FAILED};

fn exit(problems: usize) -> ExitCode {
	match problems {
		0 => ExitCode::SUCCESS,
		_ => ExitCode::from(FAILED),
	}
}

pub(crate) fn list(project: &Project) -> Result<ExitCode> {
	let host = Host::new(&project.dir, &project.descriptor)?;
	let cartridges = host.manifest().map_err(|e| {
		Error::Descriptor(format!(
			"{}: {e}",
			project.descriptor.join("init.lua").display()
		))
	})?;
	Ok(exit(lines(&cartridges)))
}

pub(crate) fn ledger(project: &Project) -> ExitCode {
	let ledger = cartridge::ledger::Ledger::scan(&project.dir);
	exit(ledger_lines(&ledger))
}

fn sends(
	all: &[CartridgeInfo],
	cartridge: &CartridgeInfo,
	depth: usize,
	stack: &mut Vec<String>,
	out: &mut Vec<String>,
) {
	let pad = "  ".repeat(depth);
	let mut names: Vec<&String> = cartridge.events.iter().collect();
	names.extend(
		cartridge
			.needs
			.iter()
			.filter(|n| !cartridge.events.contains(n)),
	);
	for key in names {
		let listeners: Vec<&CartridgeInfo> = all
			.iter()
			.filter(|p| !p.entry.disabled && p.listen.iter().any(|k| k == key))
			.collect();
		if listeners.is_empty() && cartridge.needs.contains(key) {
			out.push(format!("{pad}{key} -> ?"));
		}
		for listener in listeners {
			if stack.contains(&listener.entry.id) {
				out.push(format!("{pad}{key} -> {} (cycle)", listener.entry.id));
				continue;
			}
			out.push(format!("{pad}{key} -> {}", listener.entry.id));
			stack.push(listener.entry.id.clone());
			sends(all, listener, depth + 1, stack, out);
			stack.pop();
		}
	}
}

fn lines(cartridges: &[CartridgeInfo]) -> usize {
	for p in cartridges {
		let mut line = format!("{}  {}", p.entry.id, p.entry.path);
		if p.entry.disabled {
			line.push_str("  (disabled)");
		}
		if !p.events.is_empty() {
			line.push_str(&format!("  events {}", p.events.join(", ")));
		}
		if !p.listen.is_empty() {
			line.push_str(&format!("  listens {}", p.listen.join(", ")));
		}
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
			if grant.audio {
				line.push_str("  audio");
			}
		}
		for note in [p.unread.as_deref(), p.error.as_deref()]
			.into_iter()
			.flatten()
		{
			line.push_str(&format!("  error: {note}"));
		}
		println!("{line}");
		let mut graph = Vec::new();
		sends(cartridges, p, 1, &mut vec![p.entry.id.clone()], &mut graph);
		for line in graph {
			println!("{line}");
		}
	}
	cartridges.iter().filter(|p| p.unread.is_some()).count()
}

fn ledger_lines(ledger: &cartridge::ledger::Ledger) -> usize {
	for e in ledger.entries() {
		let mut line = e.path.clone();
		if !e.name.is_empty() && e.name != e.path.rsplit('/').next().unwrap_or("") {
			line.push_str(&format!("  ({})", e.name));
		}
		if !e.listen.is_empty() {
			line.push_str(&format!("  listens {}", e.listen.join(", ")));
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

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/cli/listing.rs"]
mod tests;
