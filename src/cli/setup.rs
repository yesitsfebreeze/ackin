use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use cartridge::host::Host;
use cartridge::ledger::Ledger;
use cartridge::loader::{Cartridge, MANIFEST};
use cartridge::{Error, Result};
use serde_json::{json, Map, Value};

use super::settings::{lua_key, lua_value};
use super::{Project, FAILED};

pub(crate) struct Ask {
	pub(crate) from: Vec<PathBuf>,
	pub(crate) with: Vec<String>,
	pub(crate) yes: bool,
	pub(crate) catalog: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate {
	pub(crate) name: String,
	pub(crate) description: String,
	pub(crate) source: Source,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
	Folder(PathBuf),
	Repository(String),
}

const INIT: &str = "init.lua";
const CONFIG: &str = "config.lua";
const IGNORE: &str = ".gitignore";
const WRITTEN_BY_SETUP: &str = "-- Written by `cartridge setup`;";
const ROUNDS: usize = 16;

pub(crate) async fn setup(root: &Path, dir: &Path, ask: Ask) -> Result<ExitCode> {
	let descriptor = root.join(".cartridge");
	if descriptor.join(INIT).is_file() {
		eprintln!(
			"{} already composes this project; edit it, or remove it to set up again",
			descriptor.join(INIT).display()
		);
		return Ok(ExitCode::from(FAILED));
	}
	let interactive = ask.with.is_empty() && !ask.yes;
	if interactive && !std::io::stdin().is_terminal() {
		return Err(Error::Argument(
			"setup asks on a terminal; without one pass --yes or --with <name>,...".into(),
		));
	}
	let mut prompt = Prompt;
	if interactive {
		println!("{} is not a project yet.", root.display());
		if !prompt.confirm("Set it up as one?", true)? {
			return Ok(ExitCode::SUCCESS);
		}
	}
	let from = match ask.from.is_empty() {
		false => ask.from.clone(),
		true if interactive => {
			let suggested = suggest(root, dir);
			let answer = prompt.line(&format!(
				"Where are cartridge folders to choose from? [{}]",
				suggested.display()
			))?;
			vec![match answer.trim() {
				"" => suggested,
				path => root.join(path),
			}]
		}
		true => vec![dir.to_path_buf()],
	};
	let catalog_path = ask.catalog.clone().map_or_else(catalog_path, Ok)?;
	let catalog = read_catalog(&catalog_path)?;
	let candidates = candidates(&from, catalog);
	if candidates.is_empty() {
		eprintln!(
			"nothing to offer: no folder holding {MANIFEST} under {}, and no catalog at {}",
			from.iter()
				.map(|p| p.display().to_string())
				.collect::<Vec<_>>()
				.join(", "),
			catalog_path.display()
		);
		return Ok(ExitCode::from(FAILED));
	}
	let chosen = if !ask.with.is_empty() {
		select_named(&candidates, &ask.with)?
	} else if ask.yes {
		candidates
	} else {
		let chosen = choose(&mut prompt, &candidates)?;
		if chosen.is_empty() {
			println!("Nothing chosen; nothing written.");
			return Ok(ExitCode::SUCCESS);
		}
		let (clone, link): (Vec<_>, Vec<_>) = chosen
			.iter()
			.partition(|c| matches!(c.source, Source::Repository(_)));
		println!(
			"Will clone {} and link {} under {}, then write {}.",
			names(&clone),
			names(&link),
			dir.display(),
			descriptor.join(INIT).display()
		);
		if !prompt.confirm("Continue?", true)? {
			return Ok(ExitCode::SUCCESS);
		}
		chosen
	};
	let installed = install(dir, &chosen)?;
	let written = write(root, dir, &installed)?;
	for line in &written {
		println!("{line}");
	}
	cartridge::settings::settle(&descriptor);
	let outcome = run_setups(root, dir, &installed, interactive).await?;
	for line in &outcome {
		println!("{line}");
	}
	println!("Next: cartridge list, cartridge settings, cartridge doctor, cartridge help.");
	Ok(ExitCode::SUCCESS)
}

fn suggest(root: &Path, dir: &Path) -> PathBuf {
	if !Ledger::scan(dir).is_empty() {
		return dir.to_path_buf();
	}
	root.parent()
		.map_or_else(|| root.to_path_buf(), Path::to_path_buf)
}

pub(crate) fn catalog_path() -> Result<PathBuf> {
	Ok(cartridge::trust::home()?.join("catalog.json"))
}

pub(crate) fn read_catalog(path: &Path) -> Result<Vec<Candidate>> {
	if !path.is_file() {
		return Ok(Vec::new());
	}
	let text = std::fs::read_to_string(path).map_err(|e| Error::file(path, e))?;
	let map: Map<String, Value> = serde_json::from_str(&text)
		.map_err(|e| Error::Settings(format!("{}: {e}", path.display())))?;
	let mut out = Vec::new();
	for (name, entry) in map {
		let Some(repository) = entry["repository"].as_str() else {
			return Err(Error::Settings(format!(
				"{}: `{name}` names no repository",
				path.display()
			)));
		};
		out.push(Candidate {
			name,
			description: entry["description"].as_str().unwrap_or_default().to_owned(),
			source: Source::Repository(repository.to_owned()),
		});
	}
	Ok(out)
}

pub(crate) fn candidates(from: &[PathBuf], catalog: Vec<Candidate>) -> Vec<Candidate> {
	let mut found: Vec<Candidate> = Vec::new();
	for dir in from {
		for entry in Ledger::scan(dir).entries() {
			if entry.path.contains('/') || entry.name.is_empty() {
				continue;
			}
			if found.iter().any(|c| c.name == entry.name) {
				continue;
			}
			let description = Cartridge::document(&entry.dir.join(MANIFEST))
				.ok()
				.and_then(|d| d.description)
				.unwrap_or_default();
			found.push(Candidate {
				name: entry.name.clone(),
				description,
				source: Source::Folder(entry.dir.clone()),
			});
		}
	}
	for known in catalog {
		if !found.iter().any(|c| c.name == known.name) {
			found.push(known);
		}
	}
	found.sort_by(|a, b| a.name.cmp(&b.name));
	found
}

fn choose(prompt: &mut Prompt, candidates: &[Candidate]) -> Result<Vec<Candidate>> {
	let mut shown: Vec<usize> = (0..candidates.len()).collect();
	loop {
		println!("{}:", plural(shown.len(), "cartridge"));
		for &i in &shown {
			let c = &candidates[i];
			let how = match &c.source {
				Source::Folder(_) => "on disk".to_owned(),
				Source::Repository(url) => format!("clone {url}"),
			};
			println!("  {:>2}  {:<16} {}  ({how})", i + 1, c.name, c.description);
		}
		let answer = prompt
			.line("Which take part? numbers or names, a word to narrow, `all`, or `none` [all]")?;
		let answer = answer.trim();
		match select(candidates, &shown, answer) {
			Ok(chosen) => return Ok(chosen),
			Err(_) => {
				let narrowed: Vec<usize> = (0..candidates.len())
					.filter(|&i| matches(&candidates[i], answer))
					.collect();
				if narrowed.is_empty() {
					println!("Nothing matches `{answer}`.");
				} else {
					shown = narrowed;
				}
			}
		}
	}
}

pub(crate) fn matches(candidate: &Candidate, query: &str) -> bool {
	let hay = format!("{} {}", candidate.name, candidate.description).to_lowercase();
	let mut chars = hay.chars();
	query
		.to_lowercase()
		.chars()
		.filter(|c| !c.is_whitespace())
		.all(|q| chars.any(|h| h == q))
}

fn select_named(candidates: &[Candidate], wanted: &[String]) -> Result<Vec<Candidate>> {
	let mut chosen: Vec<Candidate> = Vec::new();
	for name in wanted {
		let Some(c) = candidates.iter().find(|c| &c.name == name) else {
			return Err(Error::Argument(format!(
				"no cartridge named `{name}` among {}",
				names(&candidates.iter().collect::<Vec<_>>())
			)));
		};
		if !chosen.iter().any(|x| x.name == c.name) {
			chosen.push(c.clone());
		}
	}
	Ok(chosen)
}

pub(crate) fn select(
	candidates: &[Candidate],
	shown: &[usize],
	answer: &str,
) -> Result<Vec<Candidate>> {
	match answer {
		"" | "all" | "*" => return Ok(shown.iter().map(|&i| candidates[i].clone()).collect()),
		"none" => return Ok(Vec::new()),
		_ => {}
	}
	let mut names = Vec::new();
	for word in answer
		.split(|c: char| c == ',' || c.is_whitespace())
		.filter(|w| !w.is_empty())
	{
		match word.parse::<usize>() {
			Ok(n) if (1..=candidates.len()).contains(&n) => {
				names.push(candidates[n - 1].name.clone())
			}
			Ok(n) => return Err(Error::Argument(format!("no cartridge numbered {n}"))),
			Err(_) => names.push(word.to_owned()),
		}
	}
	select_named(candidates, &names)
}

pub(crate) fn install(dir: &Path, chosen: &[Candidate]) -> Result<Vec<(String, PathBuf)>> {
	std::fs::create_dir_all(dir).map_err(|e| Error::file(dir, e))?;
	let mut installed = Vec::new();
	for c in chosen {
		if !cartridge::loader::is_bare_name(&c.name) || c.name == ".cartridge" {
			return Err(Error::Argument(format!(
				"{}: not a valid cartridge name",
				c.name
			)));
		}
		let place = dir.join(&c.name);
		match &c.source {
			Source::Repository(url) => {
				if place.exists() {
					return Err(Error::Argument(format!(
						"{} exists; not cloning {url} over it",
						place.display()
					)));
				}
				println!("cloning {url} into {}", place.display());
				let status = std::process::Command::new("git")
					.args(["clone", "--quiet", url])
					.arg(&place)
					.status()
					.map_err(|e| Error::process("git", e))?;
				if !status.success() {
					return Err(Error::process("git clone", format!("{url}: {status}")));
				}
				if !place.join(MANIFEST).is_file() {
					return Err(Error::Argument(format!(
						"{url} holds no {MANIFEST} at its root; not a cartridge"
					)));
				}
			}
			Source::Folder(folder) => {
				let target = absolute(folder);
				if absolute(&place) != target {
					match std::fs::symlink_metadata(&place) {
						Ok(meta)
							if meta.file_type().is_symlink()
								&& std::fs::canonicalize(&place).ok()
									== std::fs::canonicalize(&target).ok() => {}
						Ok(_) => {
							return Err(Error::Argument(format!(
								"{} exists and is not a link to {}",
								place.display(),
								target.display()
							)))
						}
						Err(_) => {
							let relative = relative(&absolute(dir), &target);
							link(&relative, &place, &target)?;
							println!("linked {} -> {}", place.display(), relative.display());
						}
					}
				}
			}
		}
		installed.push((c.name.clone(), place));
	}
	Ok(installed)
}

fn link(original: &Path, place: &Path, target: &Path) -> Result<()> {
	#[cfg(unix)]
	{
		let _ = target;
		std::os::unix::fs::symlink(original, place).map_err(|e| Error::file(place, e))
	}
	#[cfg(windows)]
	{
		let made = match target.is_dir() {
			true => std::os::windows::fs::symlink_dir(original, place),
			false => std::os::windows::fs::symlink_file(original, place),
		};
		made.map_err(|e| {
			Error::Argument(format!(
				"{}: {e}. Linking a cartridge needs the symbolic link privilege, \
				 which Developer Mode grants.",
				place.display()
			))
		})
	}
}

pub(crate) fn write(
	root: &Path,
	dir: &Path,
	installed: &[(String, PathBuf)],
) -> Result<Vec<String>> {
	let mut done = Vec::new();
	let descriptor = root.join(".cartridge");
	std::fs::create_dir_all(&descriptor).map_err(|e| Error::file(&descriptor, e))?;
	let mut init = format!(
		"-- This project's descriptor: the cartridges that take part, one entry each.\n\
		 -- `cartridge setup` wrote it; edit it freely. `path` is the folder below\n\
		 -- the cartridge root ({}); `id` is the name config.lua and\n\
		 -- `cartridge settings` use for the entry. An entry may also carry\n\
		 -- `config` and `disabled`.\n\
		 return {{\n",
		dir.file_name().map_or(".".into(), |n| n.to_string_lossy())
	);
	for (name, _) in installed {
		init.push_str(&format!("\t{{ id = {name:?}, path = {name:?} }},\n"));
	}
	init.push_str("}\n");
	put(&descriptor.join(INIT), &init, &mut done)?;
	let kept_config = descriptor.join(CONFIG).exists();
	if !kept_config {
		put(
			&descriptor.join(CONFIG),
			&render_config(&Map::new()),
			&mut done,
		)?;
	}
	if !descriptor.join(IGNORE).exists() {
		put(
			&descriptor.join(IGNORE),
			"# Whitelist: the descriptor is tracked, what a run writes beside it is not.\n\
			 /*\n!/.gitignore\n!/init.lua\n!/config.lua\n",
			&mut done,
		)?;
	}
	let mut wrote = vec![descriptor.join(INIT)];
	if !kept_config {
		wrote.push(descriptor.join(CONFIG));
	}
	cartridge::trust::record_files(&descriptor, &wrote)?;
	for (_, place) in installed {
		cartridge::trust::record(place)?;
	}
	Ok(done)
}

fn render_config(config: &Map<String, Value>) -> String {
	let mut out = format!(
		"{WRITTEN_BY_SETUP} what each cartridge's setup answered, one table per\n\
		 -- entry id in init.lua, laid over ~/.cartridge/config.lua field by field.\n\
		 -- `cartridge settings --template` prints every key at its current value in\n\
		 -- this shape; pin only what is this project's, never a limit that restates\n\
		 -- its declared default. Setup rewrites this file only while this header stays.\n\
		 return {{\n"
	);
	for (id, table) in config {
		out.push_str(&format!("\t{} = {},\n", lua_key(id), lua_value(table)));
	}
	out.push_str("}\n");
	out
}

async fn run_setups(
	root: &Path,
	dir: &Path,
	installed: &[(String, PathBuf)],
	interactive: bool,
) -> Result<Vec<String>> {
	let asks: Vec<(String, String)> = installed
		.iter()
		.filter_map(|(name, place)| {
			let key = Cartridge::document(&place.join(MANIFEST)).ok()?.setup?;
			Some((name.clone(), key))
		})
		.collect();
	if asks.is_empty() {
		return Ok(vec![
			"No cartridge declares a setup event; nothing to ask.".into()
		]);
	}
	let descriptor = root.join(".cartridge");
	let host = Host::new(dir, &descriptor)?;
	let (config, mut report) = started(&host, || async {
		let mut config = Map::new();
		let mut report = Vec::new();
		for (id, key) in &asks {
			let mut answers = Map::new();
			let mut reply = host
				.send_to(
					id,
					key,
					json!({"answers": answers, "interactive": interactive}),
				)
				.await?;
			for _ in 0..ROUNDS {
				let Some(object) = reply.as_object() else {
					report.push(format!(
						"{id}: setup answered with no object; nothing written"
					));
					break;
				};
				let questions = object
					.get("ask")
					.and_then(Value::as_array)
					.cloned()
					.unwrap_or_default();
				let done = object.get("done").and_then(Value::as_bool).unwrap_or(false);
				if let Some(note) = object.get("note").and_then(Value::as_str) {
					report.push(format!("{id}: {note}"));
				}
				if done || questions.is_empty() {
					if let Some(table) = object.get("config").filter(|c| c.is_object()) {
						config.insert(id.clone(), table.clone());
					}
					break;
				}
				let mut stopped = false;
				for q in &questions {
					let Some(key) = q["key"].as_str() else {
						continue;
					};
					let default = q.get("default").cloned().unwrap_or(Value::Null);
					let answer = if interactive {
						tokio::task::block_in_place(|| answer(id, q, &default))?
					} else if default.is_null() {
						report.push(format!(
							"{id}: needs an answer for `{key}`; run `cartridge setup` on a terminal"
						));
						stopped = true;
						break;
					} else {
						default
					};
					answers.insert(key.to_owned(), answer);
				}
				if stopped {
					break;
				}
				reply = host
					.send_to(
						id,
						key,
						json!({"answers": answers, "interactive": interactive}),
					)
					.await?;
			}
		}
		Ok((config, report))
	})
	.await?;
	if !config.is_empty() {
		let path = descriptor.join(CONFIG);
		let existing = std::fs::read_to_string(&path).unwrap_or_default();
		if existing.is_empty() || existing.starts_with(WRITTEN_BY_SETUP) {
			let mut merged = cartridge::settings::read(&path)?;
			cartridge::settings::merge(&mut merged, Value::Object(config));
			let table = merged.as_object().cloned().unwrap_or_default();
			strayed(root, &path)?;
			std::fs::write(&path, render_config(&table)).map_err(|e| Error::file(&path, e))?;
			cartridge::trust::record(root)?;
			report.push(format!("wrote {}", path.display()));
		} else {
			report.push(format!(
				"{} was not written by setup; add this yourself:\n{}",
				path.display(),
				render_config(&config)
			));
		}
	}
	Ok(report)
}

fn strayed(root: &Path, spare: &Path) -> Result<()> {
	if let Some(file) = cartridge::trust::pending(root)?
		.iter()
		.find(|file| **file != spare)
	{
		return Err(Error::Untrusted {
			project: root.to_path_buf(),
			file: file.clone(),
			why: "changed while the setup exchange ran",
		});
	}
	Ok(())
}

async fn started<T, F, Fut>(host: &Arc<Host>, ask: F) -> Result<T>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	let body = async {
		host.reconcile().await?;
		host.settled(cartridge::settings::host().verify_timeout())
			.await;
		ask().await
	};
	let result = tokio::select! {
		result = body => result,
		() = host.stopped() => Err(cartridge::Error::Stopped),
	};
	host.stop().await;
	result
}

fn answer(id: &str, question: &Value, default: &Value) -> Result<Value> {
	let prompt = question["prompt"]
		.as_str()
		.unwrap_or_else(|| question["key"].as_str().unwrap_or(""));
	let hint = match default {
		Value::Null => String::new(),
		Value::String(s) => format!(" [{s}]"),
		other => format!(" [{other}]"),
	};
	let text = Prompt.line(&format!("{id}: {prompt}{hint}"))?;
	let text = text.trim();
	if text.is_empty() {
		return Ok(default.clone());
	}
	Ok(serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_owned())))
}

pub(crate) async fn doctor(project: &Project) -> Result<ExitCode> {
	let host = super::host::host(project, None)?;
	let composed = host.entries().map_err(|e| {
		Error::Descriptor(format!("{}: {e}", project.descriptor.join(INIT).display()))
	})?;
	let mut asks = Vec::new();
	let mut lines = Vec::new();
	for entry in composed.iter().filter(|e| !e.disabled) {
		match Cartridge::document(&entry.file(host.dir()))
			.ok()
			.and_then(|d| d.doctor)
		{
			Some(key) => asks.push((entry.id.clone(), key)),
			None => lines.push(format!("--  {}: declares no doctor event", entry.id)),
		}
	}
	let mut problems = 0;
	if !asks.is_empty() {
		let answers = started(&host, || async {
			let mut out = Vec::new();
			for (id, key) in &asks {
				out.push((id, host.send_to(id, key, json!({})).await));
			}
			Ok(out)
		})
		.await?;
		for (id, answer) in answers {
			let answer = match answer {
				Ok(answer) => answer,
				Err(error) => {
					problems += 1;
					lines.push(format!("!!  {id}: {error}"));
					continue;
				}
			};
			match answer["ok"].as_bool() {
				Some(true) => lines.push(format!("ok  {id}")),
				Some(false) => {
					problems += 1;
					let why: Vec<&str> = answer["problems"]
						.as_array()
						.into_iter()
						.flatten()
						.filter_map(Value::as_str)
						.collect();
					lines.push(format!("!!  {id}: {}", why.join("; ")));
				}
				None => {
					problems += 1;
					lines.push(format!("!!  {id}: answered {answer}, not {{ok, problems}}"));
				}
			}
		}
	}
	lines.sort_by_key(|l| !l.starts_with("!!"));
	for line in &lines {
		println!("{line}");
	}
	println!("{} asked, {} with problems", asks.len(), problems);
	Ok(match problems {
		0 => ExitCode::SUCCESS,
		_ => ExitCode::from(FAILED),
	})
}

fn put(path: &Path, text: &str, done: &mut Vec<String>) -> Result<()> {
	std::fs::write(path, text).map_err(|e| Error::file(path, e))?;
	done.push(format!("wrote {}", path.display()));
	Ok(())
}

fn names(candidates: &[&Candidate]) -> String {
	match candidates.is_empty() {
		true => "nothing".to_owned(),
		false => candidates
			.iter()
			.map(|c| c.name.as_str())
			.collect::<Vec<_>>()
			.join(", "),
	}
}

fn absolute(path: &Path) -> PathBuf {
	cartridge::trust::absolute_clean(path)
}

fn relative(from: &Path, to: &Path) -> PathBuf {
	let mut f = from.components().peekable();
	let mut t = to.components().peekable();
	while let (Some(a), Some(b)) = (f.peek(), t.peek()) {
		if a != b {
			break;
		}
		f.next();
		t.next();
	}
	let mut out = PathBuf::new();
	for _ in f {
		out.push("..");
	}
	for part in t {
		out.push(part);
	}
	out
}

fn plural(n: usize, what: &str) -> String {
	match n {
		1 => format!("1 {what}"),
		_ => format!("{n} {what}s"),
	}
}

struct Prompt;

impl Prompt {
	fn line(&mut self, question: &str) -> Result<String> {
		print!("{question} ");
		std::io::stdout()
			.flush()
			.map_err(|e| Error::file("stdout", e))?;
		let mut answer = String::new();
		std::io::stdin()
			.lock()
			.read_line(&mut answer)
			.map_err(|e| Error::file("stdin", e))?;
		Ok(answer)
	}

	fn confirm(&mut self, question: &str, default: bool) -> Result<bool> {
		let hint = if default { "[Y/n]" } else { "[y/N]" };
		let answer = self.line(&format!("{question} {hint}"))?;
		Ok(match answer.trim().to_ascii_lowercase().as_str() {
			"" => default,
			"y" | "yes" => true,
			_ => false,
		})
	}
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/cli/setup.rs"]
pub(crate) mod tests;
