//! `help`: every document this composition carries, as one tree — the host and
//! each cartridge, then the documents each one ships, then each document's
//! sections. On a terminal it is a picker that descends one level at a time;
//! anywhere else the same tree is addressed by path and printed, so an agent
//! reads exactly what a person browses.
//!
//! A module's documents are the Markdown and text files in its own directory
//! that Git does not ignore — `llms.txt`, every README, `docs/`, the memos and
//! the help page — plus its `cartridge.json` declarations rendered as one more
//! document. Nothing is authored or cached here: reading again is the update.

use std::fmt::Write as _;
use std::io::{IsTerminal, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cartridge::host::Host;
use cartridge::loader::{self, CartridgeInfo};
use cartridge::{Error, Result};
use serde_json::{json, Value};

use super::{fail, Project, FAILED};

const PAGE: &str = "help.md";
const DECLARATIONS: &str = "cartridge.json";
const HOST_ABOUT: &str =
	"the runtime itself: what a cartridge is, how one is written, how this composition is read";

pub(crate) fn help(project: &Project, what: &str, as_json: bool) -> Result<ExitCode> {
	let host = Host::new(&project.dir, &project.profile)?;
	let cartridges = host.manifest().map_err(|e| {
		Error::Profile(format!(
			"{}: {e}",
			project.profile.join("init.lua").display()
		))
	})?;
	let manual = Manual::read(project, &cartridges);
	let what = what.trim();
	let node = manual.find(what);
	if as_json {
		let (value, found) = match node {
			Some(node) => (manual.node_json(node), true),
			None => {
				let hits = manual.search(Node::Root, what);
				let found = !hits.is_empty();
				(
					json!({ "query": what, "hits": hits.iter().map(Hit::json).collect::<Vec<_>>() }),
					found,
				)
			}
		};
		println!("{value}");
		return Ok(if found {
			ExitCode::SUCCESS
		} else {
			ExitCode::from(FAILED)
		});
	}
	if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
		let (start, query) = match node {
			Some(_) => (what, ""),
			None => ("", what),
		};
		return Ok(match browse(&manual, start, query) {
			Ok(()) => ExitCode::SUCCESS,
			Err(e) => fail(FAILED, format!("help: terminal: {e}")),
		});
	}
	let problems = match node {
		Some(node) => manual.print(node),
		None => manual.print_search(what),
	};
	Ok(if problems == 0 {
		ExitCode::SUCCESS
	} else {
		ExitCode::from(FAILED)
	})
}

// ── the tree ────────────────────────────────────────────────────────────────

/// One heading inside a document: the line it starts on and how deep it sits.
struct Section {
	title: String,
	slug: String,
	line: usize,
	level: usize,
}

/// A file a module ships, read whole, with the headings found in it.
struct Doc {
	path: String,
	about: String,
	text: String,
	/// The text lowercased once, line for line, so a search per keystroke does
	/// not lowercase six megabytes each time.
	lower: String,
	sections: Vec<Section>,
}

/// The host or one cartridge: its one line and every document it carries.
struct Module {
	id: String,
	about: String,
	enabled: bool,
	dir: PathBuf,
	/// No help page, or a `cartridge.json` that would not read: what the
	/// overview reports, because a cartridge documents itself.
	broken: bool,
	docs: Vec<Doc>,
}

struct Manual {
	modules: Vec<Module>,
}

#[derive(Clone, Copy)]
enum Node<'a> {
	Root,
	Module(&'a Module),
	Doc(&'a Module, &'a Doc),
	Section(&'a Module, &'a Doc, usize),
}

/// One row a level lists: a child to descend into, or a line a search found.
struct Row {
	address: String,
	label: String,
	about: String,
	/// Set on a search hit: the line the document opens at.
	line: Option<usize>,
}

struct Hit {
	address: String,
	line: usize,
	text: String,
}

impl Hit {
	fn json(&self) -> Value {
		json!({ "address": self.address, "line": self.line + 1, "text": self.text })
	}
}

impl Manual {
	fn read(project: &Project, cartridges: &[CartridgeInfo]) -> Self {
		let root = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
		let builtin = std::fs::canonicalize(&project.dir).unwrap_or_else(|_| project.dir.clone());
		let mut modules = vec![Module {
			id: "host".into(),
			about: HOST_ABOUT.into(),
			enabled: true,
			broken: !project.profile.join(PAGE).is_file(),
			docs: documents(&root, builtin.strip_prefix(&root).ok()),
			dir: root,
		}];
		for c in cartridges {
			let manifest = c.entry.file(&project.dir);
			let home = manifest.parent().map(Path::to_path_buf).unwrap_or_default();
			let dir = std::fs::canonicalize(&home).unwrap_or(home);
			let document = loader::Cartridge::document(&manifest);
			let about = match &document {
				Ok(doc) => doc.description.clone().unwrap_or_default(),
				Err(e) => format!("error: {e}"),
			};
			let mut declared = Doc::new(DECLARATIONS.into(), declarations(c, &document, &dir));
			declared.about =
				"the events it declares, listens to and needs, its settings and commands".into();
			let mut docs = vec![declared];
			docs.extend(documents(&dir, None));
			modules.push(Module {
				id: c.entry.id.clone(),
				about,
				enabled: !c.entry.disabled,
				broken: document.is_err() || !dir.join(".cartridge").join(PAGE).is_file(),
				dir,
				docs,
			});
		}
		// Enabled first, each group in composition order.
		modules[1..].sort_by_key(|m| !m.enabled);
		Self { modules }
	}

	/// An address names a node: `<id>`, `<id>/<file>`, `<id>/<file>#<section>`.
	/// The empty address is the root. An id may itself contain a slash, so the
	/// longest id the address starts with wins.
	fn find(&self, address: &str) -> Option<Node<'_>> {
		if address.is_empty() {
			return Some(Node::Root);
		}
		let module = self
			.modules
			.iter()
			.filter(|m| address == m.id || address.starts_with(&format!("{}/", m.id)))
			.max_by_key(|m| m.id.len())?;
		let rest = address[module.id.len()..].trim_start_matches('/');
		if rest.is_empty() {
			return Some(Node::Module(module));
		}
		let (path, slug) = rest.split_once('#').unwrap_or((rest, ""));
		let doc = module.docs.iter().find(|d| d.path == path)?;
		if slug.is_empty() {
			return Some(Node::Doc(module, doc));
		}
		let section = doc.sections.iter().position(|s| s.slug == slug)?;
		Some(Node::Section(module, doc, section))
	}

	fn children(&self, node: Node<'_>) -> Vec<Row> {
		let row = |address: String, label: &str, about: &str| Row {
			address,
			label: label.into(),
			about: about.into(),
			line: None,
		};
		match node {
			Node::Root => self
				.modules
				.iter()
				.map(|m| {
					let about = if m.enabled {
						m.about.clone()
					} else {
						format!("{}  [not enabled]", m.about)
					};
					row(m.id.clone(), &m.id, &about)
				})
				.collect(),
			Node::Module(m) => m
				.docs
				.iter()
				.map(|d| row(format!("{}/{}", m.id, d.path), &d.path, &d.about))
				.collect(),
			Node::Doc(m, d) => {
				let top = d.sections.iter().map(|s| s.level).min().unwrap_or(1);
				d.sections
					.iter()
					.map(|s| {
						let label = format!("{}{}", "  ".repeat(s.level - top), s.title);
						row(format!("{}/{}#{}", m.id, d.path, s.slug), &label, "")
					})
					.collect()
			}
			Node::Section(..) => Vec::new(),
		}
	}

	/// Every line under `scope` holding all the words of `query`, in any case,
	/// addressed by the section it sits in.
	fn search(&self, scope: Node<'_>, query: &str) -> Vec<Hit> {
		let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
		if words.is_empty() {
			return Vec::new();
		}
		let mut spans: Vec<(&Module, &Doc, std::ops::Range<usize>)> = Vec::new();
		match scope {
			Node::Root => {
				for m in &self.modules {
					spans.extend(m.docs.iter().map(|d| (m, d, 0..usize::MAX)));
				}
			}
			Node::Module(m) => spans.extend(m.docs.iter().map(|d| (m, d, 0..usize::MAX))),
			Node::Doc(m, d) => spans.push((m, d, 0..usize::MAX)),
			Node::Section(m, d, i) => spans.push((m, d, d.span(i))),
		}
		let mut hits = Vec::new();
		for (m, d, span) in spans {
			for (n, (text, lower)) in d.text.lines().zip(d.lower.lines()).enumerate() {
				if span.contains(&n) && words.iter().all(|w| lower.contains(w.as_str())) {
					hits.push(Hit {
						address: d.address_of(m, n),
						line: n,
						text: text.trim().into(),
					});
				}
			}
		}
		hits
	}

	fn node_json(&self, node: Node<'_>) -> Value {
		let doc_json = |m: &Module, d: &Doc| {
			json!({
				"address": format!("{}/{}", m.id, d.path),
				"file": m.dir.join(&d.path),
				"about": d.about,
				"text": d.text,
				"sections": d.sections.iter().map(|s| json!({
					"address": format!("{}/{}#{}", m.id, d.path, s.slug),
					"title": s.title,
					"level": s.level,
					"line": s.line + 1,
				})).collect::<Vec<_>>(),
			})
		};
		let module_json = |m: &Module| {
			json!({
				"address": m.id,
				"about": m.about,
				"enabled": m.enabled,
				"dir": m.dir,
				"documents": m.docs.iter().map(|d| doc_json(m, d)).collect::<Vec<_>>(),
			})
		};
		match node {
			Node::Root => {
				json!({ "modules": self.modules.iter().map(module_json).collect::<Vec<_>>() })
			}
			Node::Module(m) => module_json(m),
			Node::Doc(m, d) => doc_json(m, d),
			Node::Section(m, d, i) => json!({
				"address": format!("{}/{}#{}", m.id, d.path, d.sections[i].slug),
				"file": m.dir.join(&d.path),
				"title": d.sections[i].title,
				"line": d.sections[i].line + 1,
				"text": d.slice(d.span(i)),
			}),
		}
	}

	// ── printed ─────────────────────────────────────────────────────────────

	/// Prints a node for a reader without a terminal. Answers how many enabled
	/// modules ship no page or no readable document when that node is the root.
	fn print(&self, node: Node<'_>) -> usize {
		match node {
			Node::Root => return self.print_overview(),
			Node::Module(m) => {
				println!("# {}  ({})\n", m.id, m.dir.display());
				if !m.about.is_empty() {
					println!("{}\n", m.about);
				}
				print_rows(&self.children(node));
				println!("\nopen one: cartridge help <address>");
			}
			Node::Doc(m, d) => {
				println!(
					"# {}/{}  ({})\n---",
					m.id,
					d.path,
					m.dir.join(&d.path).display()
				);
				println!("{}", d.text.trim_end());
			}
			Node::Section(m, d, i) => {
				let s = &d.sections[i];
				println!(
					"# {}/{}#{}  ({}:{})\n---",
					m.id,
					d.path,
					s.slug,
					m.dir.join(&d.path).display(),
					s.line + 1
				);
				println!("{}", d.slice(d.span(i)).trim_end());
			}
		}
		0
	}

	fn print_overview(&self) -> usize {
		let enabled = self.modules.iter().filter(|m| m.enabled).count() - 1;
		let parked = self.modules.len() - 1 - enabled;
		println!(
			"cartridge help: {enabled} cartridges enabled, {parked} installed and not enabled\n"
		);
		let rows = self.children(Node::Root);
		print_rows(&rows);
		println!(
			"\ngo deeper:\n  cartridge help <id>                 one module: the documents it ships\n  cartridge help <id>/<file>          one document; <id>/<file>#<section> one section\n  cartridge help <words>              every line holding all the words, with its address\n  cartridge help --json [<address>]   the same as JSON; no address is the whole manual\n  on a terminal each of these opens the picker: type to filter, enter to open, esc to go back"
		);
		let broken: Vec<&str> = self
			.modules
			.iter()
			.filter(|m| m.enabled && m.broken)
			.map(|m| m.id.as_str())
			.collect();
		if !broken.is_empty() {
			eprintln!(
				"\n{} enabled module(s) ship no .cartridge/{PAGE} or no readable document ({}): a cartridge documents itself, so the fix is a page in that cartridge",
				broken.len(),
				broken.join(", ")
			);
		}
		broken.len()
	}

	fn print_search(&self, query: &str) -> usize {
		let hits = self.search(Node::Root, query);
		for hit in &hits {
			println!("{}:{}  {}", hit.address, hit.line + 1, hit.text);
		}
		if hits.is_empty() {
			eprintln!(
				"nothing in the manual matches `{query}`; `cartridge help` lists every module"
			);
			return 1;
		}
		0
	}
}

fn print_rows(rows: &[Row]) {
	let width = rows.iter().map(|r| r.address.len()).max().unwrap_or(0);
	for r in rows {
		println!("  {:width$}  {}", r.address, r.about);
	}
}

impl Doc {
	fn new(path: String, text: String) -> Self {
		let sections = sections(&path, &text);
		let about = about(&text);
		Self {
			lower: text.to_lowercase(),
			path,
			about,
			text,
			sections,
		}
	}

	/// The lines a section covers: from its heading to the next heading at its
	/// depth or shallower.
	fn span(&self, i: usize) -> std::ops::Range<usize> {
		let s = &self.sections[i];
		let end = self.sections[i + 1..]
			.iter()
			.find(|next| next.level <= s.level)
			.map_or(usize::MAX, |next| next.line);
		s.line..end
	}

	fn slice(&self, span: std::ops::Range<usize>) -> String {
		self.text
			.lines()
			.enumerate()
			.filter(|(n, _)| span.contains(n))
			.map(|(_, l)| l)
			.collect::<Vec<_>>()
			.join("\n")
	}

	fn address_of(&self, m: &Module, line: usize) -> String {
		match self.sections.iter().rev().find(|s| s.line <= line) {
			Some(s) => format!("{}/{}#{}", m.id, self.path, s.slug),
			None => format!("{}/{}", m.id, self.path),
		}
	}
}

/// The Markdown and text files under `dir` that Git does not ignore, tracked or
/// not, ordered so a module's own page and README come first. Outside a Git
/// checkout the directory is walked instead, hidden folders other than
/// `.cartridge` skipped. `skip` leaves out one subtree: the cartridge root,
/// whose cartridges are modules of their own.
fn documents(dir: &Path, skip: Option<&Path>) -> Vec<Doc> {
	let listed = std::process::Command::new("git")
		.arg("-C")
		.arg(dir)
		.args([
			"ls-files",
			"-z",
			"--cached",
			"--others",
			"--exclude-standard",
			"--",
			"*.md",
			"*.txt",
		])
		.stderr(std::process::Stdio::null())
		.output()
		.ok()
		.filter(|out| out.status.success())
		.map(|out| {
			String::from_utf8_lossy(&out.stdout)
				.split('\0')
				.filter(|p| !p.is_empty())
				.map(PathBuf::from)
				.collect::<Vec<_>>()
		});
	let mut paths = listed.unwrap_or_else(|| {
		let mut found = Vec::new();
		walk(dir, Path::new(""), &mut found);
		found
	});
	if let Some(skip) = skip.filter(|s| !s.as_os_str().is_empty()) {
		paths.retain(|p| !p.starts_with(skip));
	}
	paths.sort_by_key(|p| (rank(p), p.clone()));
	paths.dedup();
	paths
		.into_iter()
		.filter_map(|p| {
			let text = std::fs::read_to_string(dir.join(&p)).ok()?;
			Some(Doc::new(p.to_string_lossy().into_owned(), text))
		})
		.collect()
}

fn walk(dir: &Path, rel: &Path, found: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(dir.join(rel)) else {
		return;
	};
	for entry in entries.flatten() {
		let name = entry.file_name();
		let name = name.to_string_lossy();
		let path = rel.join(&*name);
		if entry.file_type().is_ok_and(|t| t.is_dir()) {
			if (name.starts_with('.') && name != ".cartridge")
				|| name == "target"
				|| name == "node_modules"
			{
				continue;
			}
			walk(dir, &path, found);
		} else if name.ends_with(".md") || name.ends_with(".txt") {
			found.push(path);
		}
	}
}

/// Reading order inside a module: its page, its README, its agent guide, its
/// guides, then everything else, memos last.
fn rank(path: &Path) -> u8 {
	let p = path.to_string_lossy();
	match p.as_ref() {
		".cartridge/help.md" => 0,
		"README.md" => 1,
		"llms.txt" => 2,
		_ if p.starts_with("docs/") => 3,
		_ if p.starts_with(".cartridge/memos/") => 5,
		_ => 4,
	}
}

/// A module's `cartridge.json` as a reader wants it: what it declares, with
/// `needs` globs expanded.
fn declarations(c: &CartridgeInfo, document: &Result<loader::Cartridge>, dir: &Path) -> String {
	// Not a heading: the declarations are one leaf, addressed by the file alone.
	let mut out = format!(
		"{}  ({} -> {}){}\n\n",
		c.entry.id,
		c.entry.path,
		dir.display(),
		if c.entry.disabled {
			"  [installed, not enabled]"
		} else {
			""
		}
	);
	let doc = match document {
		Ok(doc) => doc,
		Err(e) => {
			let _ = writeln!(out, "document: error: {e}");
			return out;
		}
	};
	if let Some(d) = &doc.description {
		let _ = writeln!(out, "{d}\n");
	}
	let events: Vec<String> = doc.events.keys().cloned().collect();
	for (label, keys) in [
		("events", &events),
		("needs", &doc.needs),
		("listens", &doc.listen),
		("wired", &c.needs),
	] {
		if !keys.is_empty() {
			let _ = writeln!(out, "{label}: {}", keys.join(", "));
		}
	}
	if !doc.settings.is_empty() {
		let _ = writeln!(out, "settings:  (cartridge settings {})", c.entry.id);
		for (key, spec) in &doc.settings {
			let _ = writeln!(out, "  {key:24} {}", spec.doc.as_deref().unwrap_or(""));
		}
	}
	if !doc.commands.is_empty() {
		let _ = writeln!(out, "commands:  (run from {})", dir.display());
		for (label, cmd) in &doc.commands {
			let _ = write!(out, "  {label:8} {}", cmd.argv.join(" "));
			if cmd.cwd != "." {
				let _ = write!(out, "  (cwd {})", cmd.cwd);
			}
			if let Some(d) = &cmd.description {
				let _ = write!(out, "  # {d}");
			}
			out.push('\n');
		}
	}
	out
}

/// The headings of a document. Markdown `#` headings outside code fences and
/// front matter; in a `.txt` guide also an unindented all-capitals line after a
/// blank one, which is how those guides title their parts.
fn sections(path: &str, text: &str) -> Vec<Section> {
	let prose = path.ends_with(".txt");
	let mut out: Vec<Section> = Vec::new();
	let mut front = text.starts_with("---\n");
	let mut fence = false;
	let mut blank_before = true;
	for (n, line) in text.lines().enumerate() {
		let line = line.trim_end();
		if front {
			front = n == 0 || line != "---";
			continue;
		}
		if line.starts_with("```") || line.starts_with("~~~") {
			fence = !fence;
		}
		let heading = if fence {
			None
		} else if let Some((level, title)) = markdown_heading(line) {
			Some((level, title))
		} else if prose && blank_before && capitals(line) {
			Some((2, line.to_string()))
		} else {
			None
		};
		blank_before = line.trim().is_empty();
		let Some((level, title)) = heading else {
			continue;
		};
		let base = slug(&title);
		let mut slug = base.clone();
		let mut k = 2;
		while out.iter().any(|s| s.slug == slug) {
			slug = format!("{base}-{k}");
			k += 1;
		}
		out.push(Section {
			title,
			slug,
			line: n,
			level,
		});
	}
	out
}

fn markdown_heading(line: &str) -> Option<(usize, String)> {
	let level = line.chars().take_while(|&c| c == '#').count();
	let title = line[level..].strip_prefix(' ')?;
	((1..=6).contains(&level))
		.then(|| (level, title.trim().trim_end_matches('#').trim().to_string()))
}

fn capitals(line: &str) -> bool {
	line.len() >= 3
		&& !line.starts_with(char::is_whitespace)
		&& line.chars().any(char::is_alphabetic)
		&& !line.chars().any(char::is_lowercase)
}

fn slug(title: &str) -> String {
	let mut out = String::new();
	for c in title.to_lowercase().chars() {
		if c.is_alphanumeric() {
			out.push(c);
		} else if !out.ends_with('-') && !out.is_empty() {
			out.push('-');
		}
	}
	let out = out.trim_end_matches('-');
	if out.is_empty() {
		"section".into()
	} else {
		out.into()
	}
}

/// A document's one line: its front matter `description`, else its first
/// heading, else its first line.
fn about(text: &str) -> String {
	let mut lines = text.lines();
	if text.starts_with("---\n") {
		lines.next();
		for line in lines.by_ref() {
			if line.trim_end() == "---" {
				break;
			}
			if let Some(d) = line.strip_prefix("description:") {
				return d.trim().trim_matches('"').to_string();
			}
		}
	}
	let first = lines.map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
	first
		.trim_start_matches('#')
		.trim()
		.chars()
		.take(100)
		.collect()
}

// ── the picker ──────────────────────────────────────────────────────────────

/// The terminal, raw and on the alternate screen for as long as this lives.
struct Screen;

impl Screen {
	fn open() -> std::io::Result<Self> {
		crossterm::terminal::enable_raw_mode()?;
		crossterm::execute!(
			std::io::stdout(),
			crossterm::terminal::EnterAlternateScreen,
			crossterm::cursor::Hide
		)?;
		Ok(Self)
	}
}

impl Drop for Screen {
	fn drop(&mut self) {
		let _ = crossterm::execute!(
			std::io::stdout(),
			crossterm::cursor::Show,
			crossterm::terminal::LeaveAlternateScreen
		);
		let _ = crossterm::terminal::disable_raw_mode();
	}
}

/// One level of the descent: where it is, what is typed, what is selected.
struct Level {
	address: String,
	query: String,
	selected: usize,
}

enum Key {
	Char(char),
	Back,
	Erase,
	Up(usize),
	Down(usize),
	Open,
	Quit,
	Other,
}

fn key() -> std::io::Result<Key> {
	use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
	loop {
		let Event::Key(k) = crossterm::event::read()? else {
			// A resize, a focus change: redraw at the new size.
			return Ok(Key::Other);
		};
		if k.kind != KeyEventKind::Press {
			continue;
		}
		let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
		return Ok(match k.code {
			KeyCode::Char('c') if ctrl => Key::Quit,
			KeyCode::Char('p' | 'k') if ctrl => Key::Up(1),
			KeyCode::Char('n' | 'j') if ctrl => Key::Down(1),
			KeyCode::Char('u') if ctrl => Key::Up(10),
			KeyCode::Char('d') if ctrl => Key::Down(10),
			KeyCode::Char(c) if !ctrl => Key::Char(c),
			KeyCode::Up => Key::Up(1),
			KeyCode::Down => Key::Down(1),
			KeyCode::PageUp => Key::Up(10),
			KeyCode::PageDown => Key::Down(10),
			KeyCode::Enter | KeyCode::Right | KeyCode::Tab => Key::Open,
			KeyCode::Esc | KeyCode::Left => Key::Back,
			KeyCode::Backspace => Key::Erase,
			_ => Key::Other,
		});
	}
}

/// Descends from `start`: each level lists its children, typing filters them
/// and, from two characters on, adds every line below that level holding the
/// typed words. Enter opens a child a level deeper, or a document at the line.
fn browse(manual: &Manual, start: &str, query: &str) -> std::io::Result<()> {
	let _screen = Screen::open()?;
	let mut stack = vec![Level {
		address: start.into(),
		query: query.into(),
		selected: 0,
	}];
	// A leaf address opens straight into its document, then steps back to the
	// level that lists it.
	if !is_branch(manual.find(start)) {
		if !view(manual, start, None)? {
			return Ok(());
		}
		stack[0].address = up(manual, start);
	}
	loop {
		let level = stack.last_mut().expect("the root level is never popped");
		let node = manual.find(&level.address).unwrap_or(Node::Root);
		let rows = rows(manual, node, &level.query);
		level.selected = level.selected.min(rows.len().saturating_sub(1));
		draw_list(level, &rows)?;
		match key()? {
			Key::Quit => return Ok(()),
			Key::Char(c) => {
				level.query.push(c);
				level.selected = 0;
			}
			Key::Erase if !level.query.is_empty() => {
				level.query.pop();
				level.selected = 0;
			}
			Key::Back if !level.query.is_empty() => level.query.clear(),
			Key::Back | Key::Erase => {
				if stack.len() == 1 {
					if stack[0].address.is_empty() {
						return Ok(());
					}
					stack[0] = Level {
						address: up(manual, &stack[0].address),
						query: String::new(),
						selected: 0,
					};
				} else {
					stack.pop();
				}
			}
			Key::Up(n) => level.selected = level.selected.saturating_sub(n),
			Key::Down(n) => level.selected += n,
			Key::Open => {
				let Some(row) = rows.get(level.selected) else {
					continue;
				};
				if row.line.is_none() && is_branch(manual.find(&row.address)) {
					stack.push(Level {
						address: row.address.clone(),
						query: String::new(),
						selected: 0,
					});
				} else if !view(manual, &row.address, row.line)? {
					return Ok(());
				}
			}
			Key::Other => {}
		}
	}
}

/// A level's rows: the children the typed words match, then the lines below it
/// that hold them.
fn rows(manual: &Manual, node: Node<'_>, query: &str) -> Vec<Row> {
	let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
	let mut rows: Vec<Row> = manual
		.children(node)
		.into_iter()
		.filter(|r| {
			let hay = format!("{} {}", r.label, r.about).to_lowercase();
			words.iter().all(|w| hay.contains(w.as_str()))
		})
		.collect();
	if query.trim().len() >= 2 {
		rows.extend(manual.search(node, query).into_iter().map(|h| Row {
			label: format!("{}:{}", h.address, h.line + 1),
			about: h.text,
			address: h.address,
			line: Some(h.line),
		}));
	}
	rows
}

fn is_branch(node: Option<Node<'_>>) -> bool {
	match node {
		Some(Node::Root | Node::Module(_)) => true,
		Some(Node::Doc(_, d)) => d.sections.len() >= 2,
		_ => false,
	}
}

/// The nearest level above `address` that lists something: a section's
/// document, a document's module, a module's root. A document path holds
/// slashes of its own, so the step is taken on the tree, not the string.
fn up(manual: &Manual, address: &str) -> String {
	let above = match manual.find(address) {
		Some(Node::Section(m, d, _)) => format!("{}/{}", m.id, d.path),
		Some(Node::Doc(m, _)) => m.id.clone(),
		_ => return String::new(),
	};
	if is_branch(manual.find(&above)) {
		above
	} else {
		up(manual, &above)
	}
}

fn draw_list(level: &Level, rows: &[Row]) -> std::io::Result<()> {
	use crossterm::style::{Attribute, Print, SetAttribute};
	use crossterm::{cursor::MoveTo, queue, terminal};
	let (w, h) = terminal::size()?;
	let (w, h) = (usize::from(w), usize::from(h));
	let body = h.saturating_sub(3).max(1);
	let top = level.selected.saturating_sub(body - 1);
	let label_width = rows
		.iter()
		.map(|r| r.label.chars().count())
		.max()
		.unwrap_or(0)
		.min(w / 2);
	let mut out = std::io::stdout().lock();
	queue!(out, terminal::Clear(terminal::ClearType::All), MoveTo(0, 0))?;
	let crumb = if level.address.is_empty() {
		"help"
	} else {
		&level.address
	};
	queue!(
		out,
		SetAttribute(Attribute::Bold),
		Print(fit(&format!("{crumb} › {}", level.query), w)),
		SetAttribute(Attribute::Reset)
	)?;
	for (n, row) in rows.iter().enumerate().skip(top).take(body) {
		queue!(out, MoveTo(0, (n - top + 2) as u16))?;
		if n == level.selected {
			queue!(out, SetAttribute(Attribute::Reverse))?;
		}
		let label: String = row.label.chars().take(label_width).collect();
		let pad = label_width - label.chars().count();
		queue!(
			out,
			Print(fit(
				&format!("{label}{}  {}", " ".repeat(pad), row.about),
				w
			)),
			SetAttribute(Attribute::Reset)
		)?;
	}
	queue!(
		out,
		MoveTo(0, (h - 1) as u16),
		SetAttribute(Attribute::Dim),
		Print(fit(
			&format!(
				"{} · type to filter · enter open · esc back · ctrl-c quit",
				rows.len()
			),
			w
		)),
		SetAttribute(Attribute::Reset)
	)?;
	out.flush()
}

/// Shows a document from a line, or a section's heading when no line is given.
/// Answers false when the reader quit the whole picker rather than stepping back.
fn view(manual: &Manual, address: &str, line: Option<usize>) -> std::io::Result<bool> {
	use crossterm::style::{Attribute, Print, SetAttribute};
	use crossterm::{cursor::MoveTo, queue, terminal};
	let (doc, mark) = match manual.find(address) {
		Some(Node::Doc(_, d)) => (d, line),
		Some(Node::Section(_, d, i)) => (d, Some(line.unwrap_or(d.sections[i].line))),
		_ => return Ok(true),
	};
	let lines: Vec<&str> = doc.text.lines().collect();
	let mut top = mark.unwrap_or(0).saturating_sub(2);
	loop {
		let (w, h) = terminal::size()?;
		let (w, h) = (usize::from(w), usize::from(h));
		let body = h.saturating_sub(2).max(1);
		top = top.min(lines.len().saturating_sub(body));
		let mut out = std::io::stdout().lock();
		queue!(out, terminal::Clear(terminal::ClearType::All), MoveTo(0, 0))?;
		queue!(
			out,
			SetAttribute(Attribute::Bold),
			Print(fit(address, w)),
			SetAttribute(Attribute::Reset)
		)?;
		for (n, text) in lines.iter().enumerate().skip(top).take(body) {
			queue!(out, MoveTo(0, (n - top + 1) as u16))?;
			if Some(n) == mark {
				queue!(out, SetAttribute(Attribute::Reverse))?;
			}
			queue!(out, Print(fit(text, w)), SetAttribute(Attribute::Reset))?;
		}
		let shown = (top + body).min(lines.len());
		queue!(
			out,
			MoveTo(0, (h - 1) as u16),
			SetAttribute(Attribute::Dim),
			Print(fit(
				&format!(
					"{shown}/{} · j k scroll · space b page · g G ends · esc back · q quit",
					lines.len()
				),
				w
			)),
			SetAttribute(Attribute::Reset)
		)?;
		out.flush()?;
		drop(out);
		match key()? {
			Key::Quit | Key::Char('q') => return Ok(false),
			Key::Back | Key::Erase => return Ok(true),
			Key::Up(n) => top = top.saturating_sub(n),
			Key::Down(n) => top += n,
			Key::Char('k') => top = top.saturating_sub(1),
			Key::Char('j') => top += 1,
			Key::Char('b') => top = top.saturating_sub(body),
			Key::Char(' ') => top += body,
			Key::Char('g') => top = 0,
			Key::Char('G') => top = usize::MAX,
			_ => {}
		}
	}
}

/// One line cut to the terminal's width, tabs spread. Counts characters, not
/// display cells.
// ponytail: wide glyphs (CJK, emoji) overrun the edge; measure cells if a document carries them.
fn fit(text: &str, width: usize) -> String {
	text.replace('\t', "    ").chars().take(width).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn module(docs: Vec<Doc>) -> Manual {
		Manual {
			modules: vec![Module {
				id: "memo".into(),
				about: "typed records".into(),
				enabled: true,
				dir: PathBuf::from("/memo"),
				broken: false,
				docs,
			}],
		}
	}

	/// The tree a person descends is the tree an address names: every row a
	/// level lists resolves back to a node, and a heading is found in both a
	/// Markdown memo and a capitals-titled text guide.
	#[test]
	fn addresses_resolve_and_search_lands_in_the_section() {
		let guide = "# Memos\n\nintro\n\nRECORD FORMAT\n\nA memo is Markdown.\n\nDISCOVERY\n\nresolve finds references.\n";
		let memo = "---\nkind: note\ndescription: why memos\n---\n\n# Why\n\n```\n# not a heading\n```\n## Detail\n";
		let manual = module(vec![
			Doc::new("docs/memos.txt".into(), guide.into()),
			Doc::new("note/why.md".into(), memo.into()),
		]);

		let guide_rows = manual.children(manual.find("memo/docs/memos.txt").unwrap());
		let titles: Vec<&str> = guide_rows.iter().map(|r| r.label.trim()).collect();
		assert_eq!(titles, ["Memos", "RECORD FORMAT", "DISCOVERY"]);
		for row in manual
			.children(Node::Root)
			.iter()
			.chain(&manual.children(manual.find("memo").unwrap()))
			.chain(&guide_rows)
		{
			assert!(
				manual.find(&row.address).is_some(),
				"{} does not resolve",
				row.address
			);
		}

		let memo = manual.find("memo/note/why.md").unwrap();
		assert_eq!(
			manual.children(memo).len(),
			2,
			"the fenced line is not a heading"
		);
		assert_eq!(manual.children(Node::Root)[0].about, "typed records");
		assert_eq!(manual.modules[0].docs[1].about, "why memos");

		let hits = manual.search(Node::Root, "MARKDOWN memo");
		assert_eq!(hits.len(), 1);
		assert_eq!(hits[0].address, "memo/docs/memos.txt#record-format");
		let Some(Node::Section(_, d, i)) = manual.find("memo/docs/memos.txt#record-format") else {
			panic!("section address");
		};
		assert_eq!(d.slice(d.span(i)), "RECORD FORMAT\n\nA memo is Markdown.\n");
		assert_eq!(
			up(&manual, "memo/docs/memos.txt#discovery"),
			"memo/docs/memos.txt"
		);
		assert_eq!(up(&manual, "memo/docs/memos.txt"), "memo");
		assert_eq!(up(&manual, "memo/note/why.md#detail"), "memo/note/why.md");
	}
}
