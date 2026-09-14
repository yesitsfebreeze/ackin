//! `setup` and `doctor`: the installation tool.
//!
//! `setup` makes the working directory a project. It offers what it can see —
//! cartridge folders under a directory, and the repositories the person's
//! catalog knows — takes the ones chosen, clones or links each under the
//! cartridge root, writes the `.cartridge/init.lua` that names them, and then
//! lets each installed cartridge that declares a `setup` event ask what this
//! project must decide. `doctor` asks every composed cartridge that declares a
//! `doctor` event whether it is healthy here.
//!
//! The host knows no cartridge by name, so there is no list here of what a
//! project "should" have: what setup offers is what it found, and what it
//! writes is what was chosen. Bootstrapping is choosing.
//!
//! **The setup exchange.** The host sends the cartridge its `setup` event with
//! `{"answers": {...}, "interactive": bool}` and reads back an object:
//! `ask` is a list of `{"key", "prompt", "default"?}` the person is asked,
//! their answers joining `answers` on the next call; `config` is the table to
//! write under the entry's id in `config.lua`; `done` ends the exchange
//! (an empty `ask` does too); `note` is printed as it is. Without a terminal
//! every question takes its default, and one without a default stops the
//! exchange and says so.
//!
//! **The doctor answer.** `{"ok": bool, "problems": [text]}`; anything else
//! is reported as an answer the host did not understand.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use cartridge::host::Host;
use cartridge::ledger::Ledger;
use cartridge::loader::{Cartridge, MANIFEST};
use cartridge::{Error, Result};
use serde_json::{json, Map, Value};

use super::settings::{lua_key, lua_value};
use super::{Project, FAILED};

/// What setup was asked: where to look, what to take, and whether to ask.
pub(crate) struct Ask {
	/// Directories holding cartridge folders. Empty: the cartridge root, or
	/// the terminal is asked.
	pub(crate) from: Vec<PathBuf>,
	/// Cartridge names to take without asking.
	pub(crate) with: Vec<String>,
	/// Take everything offered without asking.
	pub(crate) yes: bool,
	/// The catalog of known repositories; absent, the person's own.
	pub(crate) catalog: Option<PathBuf>,
}

/// One cartridge setup can offer: on disk already, or known by repository.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Candidate {
	pub(crate) name: String,
	pub(crate) description: String,
	pub(crate) source: Source,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
	/// A folder holding `cartridge.json`, to be linked under the root.
	Folder(PathBuf),
	/// A repository to clone under the root.
	Repository(String),
}

/// The files setup writes under `.cartridge`; the folder is a project once
/// `init.lua` exists.
const INIT: &str = "init.lua";
const CONFIG: &str = "config.lua";
const IGNORE: &str = ".gitignore";
/// The line that marks a `config.lua` setup wrote and may rewrite.
const WRITTEN_BY_SETUP: &str = "-- Written by `cartridge setup`;";
/// Questions a cartridge's setup may ask before the host stops asking.
const ROUNDS: usize = 16;

pub(crate) async fn setup(root: &Path, dir: &Path, ask: Ask) -> Result<ExitCode> {
	let profile = root.join(".cartridge");
	if profile.join(INIT).is_file() {
		eprintln!(
			"{} already composes this project; edit it, or remove it to set up again",
			profile.join(INIT).display()
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
	let catalog_path = ask.catalog.clone().or_else(catalog_path);
	let catalog = match &catalog_path {
		Some(path) => read_catalog(path)?,
		None => Vec::new(),
	};
	let candidates = candidates(&from, catalog);
	if candidates.is_empty() {
		eprintln!(
			"nothing to offer: no folder holding {MANIFEST} under {}, and no catalog at {}",
			from.iter()
				.map(|p| p.display().to_string())
				.collect::<Vec<_>>()
				.join(", "),
			catalog_path.map_or("~/.cartridge/catalog.json".to_owned(), |p| p
				.display()
				.to_string())
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
			profile.join(INIT).display()
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
	cartridge::settings::settle(&profile);
	let outcome = run_setups(root, dir, &installed, interactive).await?;
	for line in &outcome {
		println!("{line}");
	}
	println!("Next: cartridge list, cartridge settings, cartridge doctor, cartridge help.");
	Ok(ExitCode::SUCCESS)
}

/// Where to look when nobody said: the cartridge root if it already holds
/// cartridges, else the directory above the project, where sibling checkouts
/// usually sit.
fn suggest(root: &Path, dir: &Path) -> PathBuf {
	if !Ledger::scan(dir).is_empty() {
		return dir.to_path_buf();
	}
	root.parent()
		.map_or_else(|| root.to_path_buf(), Path::to_path_buf)
}

/// The person's catalog of known repositories: `$CARTRIDGE_HOME/catalog.json`
/// or `~/.cartridge/catalog.json`, a map of name to `{repository, description}`.
pub(crate) fn catalog_path() -> Option<PathBuf> {
	if let Some(home) = std::env::var_os("CARTRIDGE_HOME") {
		return Some(PathBuf::from(home).join("catalog.json"));
	}
	let home = std::env::var_os("HOME")?;
	Some(PathBuf::from(home).join(".cartridge").join("catalog.json"))
}

/// The catalog read, in name order. A missing catalog is empty, not an error.
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

/// Every top-level cartridge folder under the directories named, then every
/// catalog entry not already on disk, in name order. Nested cartridges belong
/// to their parent and are not offered on their own.
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

/// The list, then the question, until the answer picks: numbers or names
/// take those, `all` and `none` say so, and anything else narrows the list
/// to the candidates it matches and asks again.
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

/// Every character of the query, in order, somewhere in the name or the
/// description, ignoring case. `agt` finds `agent`; `mod` finds `model`.
pub(crate) fn matches(candidate: &Candidate, query: &str) -> bool {
	let hay = format!("{} {}", candidate.name, candidate.description).to_lowercase();
	let mut chars = hay.chars();
	query
		.to_lowercase()
		.chars()
		.filter(|c| !c.is_whitespace())
		.all(|q| chars.any(|h| h == q))
}

/// The candidates the names pick, every name accounted for.
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

/// An answer at the prompt over the candidates shown: `all`, `none`, or
/// numbers and names mixed. Anything else is an error the prompt turns into
/// a narrowing.
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

/// Bring each chosen cartridge under the root: a repository is cloned into
/// `<root>/<name>`, a folder elsewhere is linked as `<root>/<name>`, and one
/// already under the root is left as it is. Returns the folders installed.
pub(crate) fn install(dir: &Path, chosen: &[Candidate]) -> Result<Vec<(String, PathBuf)>> {
	std::fs::create_dir_all(dir).map_err(|e| Error::file(dir, e))?;
	let mut installed = Vec::new();
	for c in chosen {
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
							std::os::unix::fs::symlink(&relative, &place)
								.map_err(|e| Error::file(&place, e))?;
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

/// Write the profile that names what was installed. Returns one line per
/// file written, for the person watching.
pub(crate) fn write(
	root: &Path,
	dir: &Path,
	installed: &[(String, PathBuf)],
) -> Result<Vec<String>> {
	let mut done = Vec::new();
	let profile = root.join(".cartridge");
	std::fs::create_dir_all(&profile).map_err(|e| Error::file(&profile, e))?;
	let mut init = format!(
		"-- This project's profile: the cartridges that take part, one entry each.\n\
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
	put(&profile.join(INIT), &init, &mut done)?;
	if !profile.join(CONFIG).exists() {
		put(
			&profile.join(CONFIG),
			&render_config(&Map::new()),
			&mut done,
		)?;
	}
	if !profile.join(IGNORE).exists() {
		put(
			&profile.join(IGNORE),
			"# Whitelist: the profile is tracked, what a run writes beside it is not.\n\
			 /*\n!/.gitignore\n!/init.lua\n!/config.lua\n",
			&mut done,
		)?;
	}
	// Choosing is the approval: what setup wrote and linked starts next.
	let project = cartridge::trust::record(root)?.project;
	for (_, place) in installed {
		let place = place.canonicalize().map_err(|e| Error::file(place, e))?;
		if !place.starts_with(&project) {
			cartridge::trust::record(&place)?;
		}
	}
	Ok(done)
}

/// `config.lua` as setup writes it: one table per entry id, rendered from
/// data, marked so a later setup knows it may rewrite it.
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

/// Let each installed cartridge that declares a `setup` event ask what this
/// project must decide, and write what it hands back into `config.lua`.
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
	let profile = root.join(".cartridge");
	let host = Host::new(dir, &profile)?;
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
		let path = profile.join(CONFIG);
		let existing = std::fs::read_to_string(&path).unwrap_or_default();
		if existing.is_empty() || existing.starts_with(WRITTEN_BY_SETUP) {
			let mut merged = cartridge::settings::read(&path)?;
			cartridge::settings::merge(&mut merged, Value::Object(config));
			let table = merged.as_object().cloned().unwrap_or_default();
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

/// Start the profile, let `ask` send to it, and stop it whatever `ask` answered.
async fn started<T, F, Fut>(host: &Arc<Host>, ask: F) -> Result<T>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	let result = async {
		host.reconcile().await?;
		// A cartridge still starting when the budget runs out answers with an
		// error of its own, which says more than a timeout here would.
		host.settled(cartridge::settings::host().verify_timeout())
			.await;
		ask().await
	}
	.await;
	host.stop().await;
	result
}

/// One question on the terminal, the default offered, the answer read as
/// JSON when it parses and as text otherwise.
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

/// `doctor`: every composed cartridge that declares a `doctor` event is asked
/// whether it is healthy here, and the answers are printed one per line.
pub(crate) async fn doctor(project: &Project) -> Result<ExitCode> {
	let host = Host::new(&project.dir, &project.profile)?;
	let composed = host
		.entries()
		.map_err(|e| Error::Profile(format!("{}: {e}", project.profile.join(INIT).display())))?;
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

/// A path made absolute lexically and cleaned of `.` and `..`, so two
/// spellings of one directory compare equal without touching the disk.
fn absolute(path: &Path) -> PathBuf {
	let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
	let mut out = PathBuf::new();
	for part in abs.components() {
		match part {
			Component::CurDir => {}
			Component::ParentDir => {
				out.pop();
			}
			other => out.push(other),
		}
	}
	out
}

/// `to` as seen from inside `from`: the link text that keeps working when
/// the whole tree moves.
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

/// Questions on the terminal, answers off stdin.
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
mod tests;
