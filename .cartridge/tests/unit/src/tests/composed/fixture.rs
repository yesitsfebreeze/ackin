//! The lab every composed body runs in: a disposable three-cartridge project,
//! a short runtime directory of its own, and a teardown that survives a panic.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use serde_json::{json, Value};

use super::super::built;

/// A project of three toy cartridges, the hosts started against it, and the
/// receipt every body leaves for the gate.
pub(crate) struct Lab {
	pub(crate) name: &'static str,
	pub(crate) root: PathBuf,
	base: PathBuf,
	binary: PathBuf,
	roots: Vec<PathBuf>,
	children: Vec<Child>,
	/// What this lab was actually asked to do. Counted here, where every
	/// command is built, so a body cannot name a verb it did not perform.
	verbs: Verbs,
}

/// The actions the four claims are about, tallied by the fixture.
#[derive(Default)]
struct Verbs {
	run: usize,
	launch: usize,
	mcp: usize,
	call: usize,
	/// Attaching verbs issued against a project with no composition: a cold
	/// start, which is claim 4's whole subject.
	cold: usize,
	/// The most client processes this lab had alive at one time.
	peak: usize,
}

/// The runtime directory has to stay short: `socket::base()` joins
/// `XDG_RUNTIME_DIR` with `cartridge` and then silently discards the result
/// when it is 48 characters or longer, falling back to `/tmp/cartridge-<uid>`
/// — which on a developer's machine holds every other session's hosts.
fn short_runtime(name: &str) -> PathBuf {
	let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
		Some(dir) => PathBuf::from(dir),
		None => PathBuf::from(format!("/tmp/cx{}", std::process::id())),
	};
	let dir = dir.join(name);
	std::fs::create_dir_all(&dir).unwrap();
	assert!(
		dir.join("cartridge").as_os_str().len() < 48,
		"runtime directory {} is too long: the host would silently fall back to /tmp/cartridge-<uid> and share this machine's live hosts",
		dir.display()
	);
	dir
}

impl Lab {
	pub(crate) fn new(name: &'static str) -> Self {
		let runtime = short_runtime(name);
		std::env::set_var("XDG_RUNTIME_DIR", &runtime);
		let binary = built(&["--bin", "cartridge"]);
		let base = runtime.join("cartridge");
		let mut lab = Self {
			name,
			root: PathBuf::new(),
			base,
			binary,
			roots: Vec::new(),
			children: Vec::new(),
			verbs: Verbs::default(),
		};
		lab.root = lab.project();
		lab
	}

	/// A second project, for the negative control: a composition of its own,
	/// counted by the same helper that has to report 1 for the shared one.
	pub(crate) fn project(&mut self) -> PathBuf {
		let root = std::path::PathBuf::from(format!(
			"/tmp/cx{}-{}-{}",
			std::process::id(),
			self.name,
			self.roots.len()
		));
		std::fs::create_dir_all(root.join(".cartridge")).unwrap();
		for (id, body) in [("store", STORE), ("proxy", PROXY), ("mcp", MCP)] {
			let dir = root.join(format!("{id}.ctg"));
			std::fs::create_dir_all(&dir).unwrap();
			let manifest = json!({
				"name": id, "entry": "init.lua",
				"events": {id: {"description": format!("{id} answers")}},
				"listen": [id],
			});
			std::fs::write(dir.join("cartridge.json"), manifest.to_string()).unwrap();
			std::fs::write(dir.join("init.lua"), body.replace("@EVENT@", id)).unwrap();
		}
		std::fs::write(
			root.join(".cartridge/init.lua"),
			r#"return { { id = "store", path = "store.ctg" }, { id = "proxy", path = "proxy.ctg" }, { id = "mcp", path = "mcp.ctg" } }"#,
		)
		.unwrap();
		std::fs::write(root.join(".cartridge/config.lua"), "return {}\n").unwrap();
		Command::new(&self.binary)
			.args(["trust"])
			.arg(&root)
			.env("XDG_RUNTIME_DIR", parent_runtime())
			.current_dir(&root)
			.output()
			.unwrap();
		self.roots.push(root.clone());
		root
	}

	/// `--yolo` is refused by every subcommand but `run`, `launch` and
	/// `daemon`: it may not change a daemon that already runs.
	fn command(&mut self, root: &Path, args: &[&str]) -> Command {
		let verb = args.first().copied().unwrap_or("");
		// Cold is measured before the command runs, not claimed after it.
		let attaching = matches!(verb, "run" | "launch" | "mcp" | "call");
		let cold = attaching && self.compositions(root) == 0;
		match verb {
			"run" => self.verbs.run += 1,
			"launch" => self.verbs.launch += 1,
			"mcp" => self.verbs.mcp += 1,
			"call" => self.verbs.call += 1,
			_ => {}
		}
		if cold {
			self.verbs.cold += 1;
		}
		let mut command = Command::new(&self.binary);
		if matches!(args.first(), Some(&"run" | &"launch" | &"daemon")) {
			command.arg("--yolo");
		}
		command
			.args(args)
			.current_dir(root)
			.env("XDG_RUNTIME_DIR", parent_runtime())
			.env("CARTRIDGE_YOLO", "1");
		command
	}

	/// The daemon of one project, started as this process's child. `call` and
	/// `status` never start one; only `run`, `launch` and `mcp` attach.
	pub(crate) fn start(&mut self, root: &Path) -> u32 {
		let pid = self.spawn(root, &["daemon"]);
		self.settled(root);
		pid
	}

	pub(crate) fn cli(&mut self, root: &Path, args: &[&str]) -> Output {
		self.command(root, args).output().unwrap()
	}

	pub(crate) fn call(&mut self, root: &Path, id: &str, payload: &str) -> Value {
		let out = self.cli(root, &["call", id, payload]);
		assert!(
			out.status.success(),
			"call {id} {payload}: {}",
			String::from_utf8_lossy(&out.stderr)
		);
		serde_json::from_slice(&out.stdout).unwrap()
	}

	pub(crate) fn spawn(&mut self, root: &Path, args: &[&str]) -> u32 {
		let child = self
			.command(root, args)
			.stdin(Stdio::piped())
			// Null, not piped: a launched program is reparented to init and
			// would hold this test's pipes open after it exits, which nextest
			// reports as a leaky pass. Nothing here reads a spawned client's
			// output — only its pid and whether it is alive.
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.unwrap();
		let pid = child.id();
		self.children.push(child);
		let mut alive = 0;
		for child in &mut self.children {
			if matches!(child.try_wait(), Ok(None)) {
				alive += 1;
			}
		}
		self.verbs.peak = self.verbs.peak.max(alive);
		pid
	}

	/// The run directory of one project: `base()/tag(descriptor)`. Node sockets
	/// live one level down, in a directory named by the host's own pid; nothing
	/// about a composition is recorded in the project's own `.cartridge`.
	pub(crate) fn run_dir(&self, root: &Path) -> PathBuf {
		let dir = crate::host::socket::run_dir(&root.join(".cartridge")).unwrap();
		assert!(
			dir.starts_with(&self.base),
			"run directory {} is not under the lab's own base {}: this suite would be counting, and stopping, hosts that belong to other sessions",
			dir.display(),
			self.base.display()
		);
		dir
	}

	/// How many compositions this project has: one per host-pid directory that
	/// holds a node socket.
	pub(crate) fn compositions(&self, root: &Path) -> usize {
		let run = self.run_dir(root);
		let Ok(entries) = std::fs::read_dir(&run) else {
			return 0;
		};
		entries
			.flatten()
			.filter(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
			.filter(|e| {
				std::fs::read_dir(e.path())
					.is_ok_and(|mut d| d.any(|f| f.is_ok_and(|f| is_socket(&f))))
			})
			.count()
	}

	/// The nodes of one composition: one socket per composed cartridge.
	pub(crate) fn nodes(&self, root: &Path) -> usize {
		let run = self.run_dir(root);
		let Ok(entries) = std::fs::read_dir(&run) else {
			return 0;
		};
		entries
			.flatten()
			.filter(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
			.map(|e| {
				std::fs::read_dir(e.path())
					.map(|d| d.flatten().filter(is_socket).count())
					.unwrap_or(0)
			})
			.max()
			.unwrap_or(0)
	}

	/// The pid of the host serving this project, read from the socket it owns.
	pub(crate) fn daemon(&self, root: &Path) -> u32 {
		let run = self.run_dir(root);
		std::fs::read_dir(&run)
			.unwrap()
			.flatten()
			.filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
			.find(|pid| {
				std::fs::read_dir(run.join(pid.to_string()))
					.is_ok_and(|mut d| d.any(|f| f.is_ok_and(|f| is_socket(&f))))
			})
			.unwrap_or(0)
	}

	pub(crate) fn settled(&mut self, root: &Path) -> usize {
		for _ in 0..600 {
			let out = self.cli(root, &["status"]);
			if out.status.success() {
				if let Ok(all) = serde_json::from_slice::<Vec<Value>>(&out.stdout) {
					let active = all.iter().filter(|s| s["state"] == "active").count();
					if active == 3 {
						return active;
					}
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(100));
		}
		panic!("the composition never settled for {}", root.display());
	}

	/// The receipt the gate reads. The body authors no field — not a number, not
	/// a count: `Drop` writes them from the fixture's own measurements, so a
	/// receipt exists on the panic path too, which is what lets a mutant be
	/// judged by the receipts its own failing run produced. The body supplies
	/// only its own label, and decides whether a `Lab` (and so a receipt)
	/// exists at all. A body cannot hand in a count, so a body that composed
	/// nothing writes `compositions=0` and the gate rejects it without any
	/// list of forbidden calls.
	fn receipt(&mut self) {
		let label = self.name;
		let root = self.root.clone();
		// The probe is the fixture's own, not the body's: one process ingests a
		// word only this lab knows, a second process queries it back. It is
		// answered at all only by a live host, and answered `same` only when one
		// node served both client processes.
		let word = format!("probe-{}", self.name);
		let wrote = self.try_call(
			&root,
			"store",
			&format!(r#"{{"op":"ingest","text":"{word}"}}"#),
		);
		let read = self.try_call(
			&root,
			"store",
			&format!(r#"{{"op":"query","text":"{word}"}}"#),
		);
		let probe = match (&wrote, &read) {
			(Some(w), Some(r)) => {
				let hit = if r["hit"] == true { "hit" } else { "miss" };
				let node = if r["node"] == w["node"] {
					"same"
				} else {
					"differs"
				};
				format!("probe={hit} probe_node={node}")
			}
			_ => "probe=unanswered probe_node=none".to_owned(),
		};
		// The claim LINKAGES, measured here for the same reason the probe is:
		// a body that performs the verbs but checks nothing still cannot make
		// these say `same` or `2`.
		//   run_node — claim 1: does a `cartridge run` land on the node a plain
		//   `call` already reached, or on one of its own?
		//   launched — claim 2: how many launches did the ONE proxy node see?
		let run_node = match (
			self.try_call(&root, "store", r#"{"op":"query","text":"linkage"}"#),
			self.try_run(&root, "store", r#"{"op":"query","text":"linkage"}"#),
		) {
			(Some(c), Some(r)) if r["node"] == c["node"] => "same",
			(Some(_), Some(_)) => "differs",
			_ => "none",
		};
		let launched = self
			.try_call(&root, "proxy", r#"{"op":"count"}"#)
			.and_then(|v| v["count"].as_u64())
			.map(|n| n.to_string())
			.unwrap_or_else(|| "none".to_owned());
		let live = self
			.roots
			.clone()
			.iter()
			.filter(|r| self.compositions(r) == 1)
			.count();
		let fields = format!(
			"label={label} roots={} live={live} compositions={} nodes={} daemon={} {probe} \
			 run={} launch={} mcp={} call={} cold={} peak={} run_node={run_node} launched={launched}",
			self.roots.len(),
			self.compositions(&root),
			self.nodes(&root),
			self.daemon(&root),
			self.verbs.run,
			self.verbs.launch,
			self.verbs.mcp,
			self.verbs.call,
			self.verbs.cold,
			self.verbs.peak,
		);
		let Ok(dir) = std::env::var("CARTRIDGE_COMPOSED_REPORT") else {
			return;
		};
		// One file per test, never one shared append: nextest runs each body in
		// its own process, and two appends to one file interleave (measured).
		let dir = PathBuf::from(dir);
		std::fs::create_dir_all(&dir).unwrap();
		let mut file = std::fs::File::create(dir.join(self.name)).unwrap();
		writeln!(file, "test={} {fields}", self.name).unwrap();
	}

	/// A `run` that may have no host to answer it. `run` attaches like any
	/// instance; what matters is which node answers it.
	fn try_run(&mut self, root: &Path, id: &str, payload: &str) -> Option<Value> {
		let out = self.cli(root, &["run", id, payload]);
		if !out.status.success() {
			return None;
		}
		serde_json::from_slice(&out.stdout).ok()
	}

	/// A call that may have no host to answer it.
	fn try_call(&mut self, root: &Path, id: &str, payload: &str) -> Option<Value> {
		let out = self.cli(root, &["call", id, payload]);
		if !out.status.success() {
			return None;
		}
		serde_json::from_slice(&out.stdout).ok()
	}
}

/// A node socket is a socket. A regular file of the same name is something
/// that was placed there, and is not a composed node.
fn is_socket(entry: &std::fs::DirEntry) -> bool {
	use std::os::unix::fs::FileTypeExt;
	entry.file_name().to_string_lossy().ends_with(".sock")
		&& entry.file_type().is_ok_and(|t| t.is_socket())
}

/// `trust` and the hosts must see the same runtime directory this lab exported.
fn parent_runtime() -> PathBuf {
	PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").unwrap())
}

impl Drop for Lab {
	/// Stopping is the documented command, never a process-name kill, and it
	/// runs on the panic path too: a body that fails must not leave a daemon or
	/// its launched children on a machine other sessions are using.
	fn drop(&mut self) {
		self.receipt();
		for root in self.roots.clone() {
			let _ = self
				.command(&root, &["stop"])
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.status();
		}
		for child in &mut self.children {
			let _ = child.kill();
			let _ = child.wait();
		}
		for root in self.roots.clone() {
			let _ = std::fs::remove_dir_all(&root);
		}
		// The lab's own runtime directory goes with it; the gate's parent
		// directory is the gate's to remove.
		let _ = std::fs::remove_dir_all(self.base.parent().unwrap_or(&self.base));
	}
}

/// Shared project state, held in the node's own memory: what one instance
/// ingests, another instance queries back only if one node served both.
const STORE: &str = r#"
local node = tostring({})
local seen = {}
cartridge.listen("@EVENT@", function(args)
	if args.op == "ingest" then seen[args.text] = true end
	if args.op == "query" then return { node = node, hit = seen[args.text] == true } end
	return { node = node, hit = false }
end)
"#;

/// The launch provider `cartridge launch` asks before it spawns anything.
const PROXY: &str = r#"
local node = tostring({})
local launches = {}
cartridge.listen("@EVENT@", function(args)
	if args.op == "count" then return { node = node, agents = launches, count = #launches } end
	launches[#launches + 1] = args.agent
	-- 20 s, not 120: Drop reaps the `cartridge launch` client, but not the
	-- program the HOST launched, which is reparented to init. The orphan is
	-- self-terminating either way; this bounds it to the length of one test.
	return { node = node, program = "/bin/sleep", args = { "20" } }
end)
"#;

const MCP: &str = r#"
local node = tostring({})
cartridge.listen("@EVENT@", function(args)
	return { node = node, echo = args }
end)
"#;
