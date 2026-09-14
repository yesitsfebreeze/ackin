use std::path::{Path, PathBuf};
use std::process::Stdio;

use cartridge::loader::{self, CartridgeInfo};
use cartridge::lua::Host;
use cartridge::runtime::Runtime;
use cartridge::socket::{self, Client};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Parser)]
#[command(
	name = "cartridge",
	about = "cartridges on a socket: run in the back, handle the events",
	// `help` is a subcommand of ours: the manual of what is composed, not clap's usage text.
	disable_help_subcommand = true
)]
struct Cli {
	/// Directory containing bundled cartridges (each with cartridge.json)
	#[arg(long, global = true)]
	dir: Option<PathBuf>,
	/// Automatic execution: bypass tool policy and skip resolver provenance recording
	#[arg(long, global = true)]
	yolo: bool,
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	Daemon,
	/// Start the harness proxy and run an agent against it: `cartridge launch claude -- -p hi`
	Launch {
		agent: String,
		#[arg(long, default_value = "auto:code")]
		model: String,
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		args: Vec<String>,
	},
	/// Serve the profile's tools to an MCP client over this terminal's stdio:
	/// `claude mcp add cartridge -- cartridge mcp`
	Mcp,
	/// Load a profile, call one service in the foreground, and dispose it
	Run {
		key: String,
		#[arg(default_value = "null")]
		args: String,
	},
	Send {
		name: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Publish one event on a stream channel
	Publish {
		channel: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Subscribe to a stream channel and print every event on it as it arrives
	Follow {
		channel: String,
	},
	Tail,
	/// Call a provided key with one JSON argument and print the reply
	Call {
		key: String,
		#[arg(default_value = "null")]
		args: String,
		/// Tag this call with a trace id; one is minted when absent
		#[arg(long)]
		trace: Option<String>,
	},
	Reload,
	Status,
	/// Enter, leave or report debug mode on the running host: `cartridge debug on`
	Debug {
		#[arg(default_value = "status", value_parser = ["on", "off", "status"])]
		state: String,
	},
	Socket,
	/// Unlink the socket files no listener answers on, left by runtimes that
	/// were killed rather than stopped
	Sweep,
	/// Cartridges of the profile and what each one needs, resolved to its provider
	List,
	/// Every cartridge installed under the cartridge root, and what each need binds to
	Ledger,
	/// The manual of this composition: what is enabled and what each cartridge
	/// is for, built from the cartridges themselves. `cartridge help <id>` is
	/// one cartridge's declarations and the page it ships; `cartridge help
	/// <word>` searches every page; `cartridge help host` is the runtime's own
	Help {
		/// A cartridge id, `host`, or a word to search for. Absent is the overview
		what: Option<String>,
		/// Print the whole manual as one JSON document
		#[arg(long)]
		json: bool,
	},
	/// Every tunable value this profile has: the host's own and each
	/// cartridge's, with what it is set to and which file settled it.
	/// Name a key or a cartridge to narrow it: `cartridge settings host`,
	/// `cartridge settings agent.max_steps`
	Settings {
		/// A cartridge id, or one dotted key under it. Absent lists everything
		what: Option<String>,
		/// Print the listing as JSON instead of a table
		#[arg(long)]
		json: bool,
		/// Print a commented `config.lua` carrying every key at its current
		/// value, ready to save as `~/.cartridge/config.lua` or the project's
		#[arg(long)]
		template: bool,
	},
	Up {
		key: String,
	},
	/// Launch one node of a chain and hand off: resolve it against the fresh
	/// ledger, read its program, and exec into it. A node coming up enters
	/// this, not a person
	Enter {
		node: String,
		/// The node paths still to launch after this one, in chain order
		#[arg(long, default_value = "[]")]
		rest: String,
	},
	/// Load the profile and run every contract its cartridges declare; name a
	/// cartridge to verify just that one, in isolation, against its own contract
	Verify {
		/// The cartridge: a ledger path from the cartridge root, or the folder
		/// it sits in
		cartridge: Option<String>,
	},
	/// Run one chain node of a Lua-entry cartridge in-host. The re-entry
	/// execs into this, so a node's program is the host itself; a person
	/// never invokes it, and the ledger's env protocol is the only way in
	#[command(hide = true)]
	Node,
}

async fn connect(profile: &std::path::Path) -> Client {
	match Client::connect(&socket::path(profile)).await {
		Ok(c) => c,
		Err(e) => {
			eprintln!("no daemon for {}: {e}", profile.display());
			std::process::exit(1);
		}
	}
}

fn random_hex(bytes: usize) -> String {
	use std::io::Read;
	let mut buf = vec![0u8; bytes];
	std::fs::File::open("/dev/urandom")
		.and_then(|mut f| f.read_exact(&mut buf))
		.expect("OS random source required");
	buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Run a rendered launch `{program, args, env, unset, cwd}` on this terminal
/// and resolve to its exit status.
async fn spawn(launch: Value) -> Result<Value, String> {
	let strings = |v: &Value| -> Vec<String> {
		v.as_array()
			.into_iter()
			.flatten()
			.filter_map(Value::as_str)
			.map(str::to_owned)
			.collect()
	};
	let program = launch["program"].as_str().ok_or("launch without program")?;
	let mut command = tokio::process::Command::new(program);
	command.args(strings(&launch["args"]));
	if let Some(cwd) = launch["cwd"].as_str() {
		command.current_dir(cwd);
	}
	for name in strings(&launch["unset"]) {
		command.env_remove(name);
	}
	if let Some(env) = launch["env"].as_object() {
		for (k, v) in env {
			command.env(k, v.as_str().unwrap_or_default());
		}
	}
	let status = command
		.status()
		.await
		.map_err(|e| format!("{program}: {e}"))?;
	Ok(json!(status.code().unwrap_or(1)))
}

/// The binary a node re-enters: this invocation's own image.
fn exe() -> PathBuf {
	std::env::current_exe().expect("a running binary knows its own image")
}

/// What a chain node runs. A document that declares a `binary` names its own
/// program, resolved the way every process component resolves its command —
/// the cartridge's own `bin/`, then beside the running cartridge, then `PATH`.
/// A document that declares none is **hosted**: the binary itself is the
/// program, and the node mode runs the cartridge's Lua component in-host.
enum Route {
	Program(PathBuf),
	Hosted,
}

/// What a chain node launches, resolved the way the ask and every re-entry
/// resolve it — once, so `hello`, the ask and the hand-off name the same
/// program. A document that would not read, and a `binary` that does not
/// exist, refuse naming the node; the ask checks every node before the first
/// one spawns.
fn node_route(node: &cartridge::ledger::Installed) -> Result<Route, String> {
	let manifest = node.dir.join(cartridge::loader::MANIFEST);
	let document = cartridge::loader::Cartridge::document(&manifest)
		.map_err(|e| format!("`{}`: {e}", node.path))?;
	match document.binary {
		Some(binary) => cartridge::cartridge::executable(&binary, &node.dir)
			.map(Route::Program)
			.map_err(|e| format!("`{}`: {e}", node.path)),
		// The cartridge has no program of its own, so the host is it: the
		// entry is resolved now, not evaluated — a missing Lua file is a
		// refusal of the ask, an entry that fails to apply is the node's.
		None => cartridge::loader::Cartridge::read(&manifest)
			.map(|_| Route::Hosted)
			.map_err(|e| format!("`{}`: {e}", node.path)),
	}
}

/// The hosted node: the program a Lua-entry cartridge's chain node runs is
/// the host itself. The re-entry execed into this mode, so this process is
/// the tree link the document's `provide` declared and nothing launched it
/// but its dependency: it runs the cartridge's Lua component in-host, then
/// re-enters the binary for the next link of the chain, holding the child it
/// spawned, so its own death takes its dependent with it. Its stdin closes
/// when the dependency that launched it goes away, and EOF is the second
/// half of that cascade.
///
/// The pid it writes under `CARTRIDGE_NODES` is the probe's observation handle,
/// not part of the mechanism: a test reads it to name the process it is
/// asserting about, exactly as the program fixture does.
async fn hosted_node() {
	use tokio::io::AsyncReadExt;

	let node = std::env::var("CARTRIDGE_NODE").unwrap_or_default();
	let chain: Vec<String> =
		serde_json::from_str(&std::env::var("CARTRIDGE_CHAIN").unwrap_or_default())
			.unwrap_or_default();
	let (cartridge, root) = match (
		std::env::var("CARTRIDGE_CARTRIDGE"),
		std::env::var("CARTRIDGE_ROOT"),
	) {
		(Ok(cartridge), Ok(root)) => (cartridge, root),
		_ => {
			eprintln!("node mode is entered, not asked: the ledger's env protocol is missing");
			std::process::exit(2);
		}
	};
	let root = PathBuf::from(root);

	if let Some(nodes) = std::env::var_os("CARTRIDGE_NODES") {
		let _ = std::fs::write(
			Path::new(&nodes).join(node.replace('/', "_")),
			std::process::id().to_string(),
		);
	}

	// The cartridge's Lua component, in-host: its document's `provide` is the
	// provision the ledger walked, and the apply registers the values the way
	// an in-host fiber does. The component's needs are **not** injections
	// here: the chain is their resolution — the dependency launched this
	// node, which is the binding the resolver already made — and carrying
	// them as unmet injections would hold the apply waiting for providers
	// that live in other processes and can never arrive in this one. The
	// binding is the **chain link** instead: the dependency serves the keys
	// it provides over the socket its ledger path derives, the node connects
	// and binds every need to a remote over it, and a `ctx:get` of a need
	// resolves to that remote — the author calls a need like any other
	// provided key, and the frames cross the socket. An unbound need (the
	// dependency link is down, the key was never the document's) keeps the
	// store's own refusal.
	let host = cartridge::lua::Host::new(cartridge::runtime::Runtime::new(), &root, &root);
	let needs = cartridge::ledger::Ledger::scan(&root)
		.get(&node)
		.map(|e| e.needs.clone())
		.unwrap_or_default();
	let dep = std::env::var("CARTRIDGE_DEP").unwrap_or_default();
	let dep = dep.trim();
	if !dep.is_empty() {
		if let Err(e) = bind_dependency(&host, &root, dep, &needs).await {
			eprintln!("`{node}`: {e}");
			std::process::exit(1);
		}
	}
	let mut component = match host.component(&root.join(&node), serde_json::Value::Null) {
		Ok(component) => component,
		Err(e) => {
			eprintln!("`{node}`: {e}");
			std::process::exit(1);
		}
	};
	component.inject.clear();
	host.runtime().ctx().cartridge(component);

	// The node serves the keys it provides to whoever looks it up — its
	// dependent connects here for its own needs, and the asker of the named
	// tool reaches the top of the tree. The socket is up before the
	// re-entry, so a dependent that binds at startup never finds the door
	// closed.
	let serve_path = cartridge::socket::node_path(&root, &node);
	tokio::spawn({
		let host = host.clone();
		async move {
			if let Err(e) = cartridge::socket::serve(host, &serve_path).await {
				eprintln!("node socket: {e}");
			}
		}
	});

	// The node comes up, then re-enters the binary for the next link: the
	// child is held, so dropping it — on exit or on kill — takes the rest of
	// the tree along, and its stdin is the pipe this node's death closes.
	// The pipe stays the kill channel and carries no frames; the calls cross
	// the sockets, so the cascade needs nothing from the wire and the wire
	// needs nothing from the pipe. This node names itself in the
	// dependency's seat: the dependent reads `CARTRIDGE_DEP` and derives the
	// socket to call its needs over.
	let mut dependent: Option<tokio::process::Child> = None;
	if let Some(next) = chain.first() {
		let rest = serde_json::to_string(&chain[1..]).expect("node paths serialize");
		let mut reentry = tokio::process::Command::new(&cartridge);
		reentry
			.args([
				"enter",
				next,
				"--rest",
				&rest,
				"--dir",
				root.to_string_lossy().as_ref(),
			])
			.env("CARTRIDGE_CARTRIDGE", &cartridge)
			.env("CARTRIDGE_ROOT", &root)
			.env("CARTRIDGE_DEP", &node)
			.stdin(Stdio::piped())
			.kill_on_drop(true);
		dependent = Some(reentry.spawn().expect("re-enter the resolver"));
	}

	// Stay up until the dependency that launched this node goes away, then
	// exit. The far end of the chain is different: nothing launched it that
	// owns it, so it has no pipe to watch and dies only when it is killed.
	// The held child goes with this node: the drop is what kills it.
	// A stop that can be caught is taken as one: the node returns from here,
	// its serving task is dropped with the runtime, and the socket goes with
	// it. Only the kill that cannot be caught leaves an entry, and the daemon's
	// sweep is what collects that.
	let gone = async {
		if std::env::var_os("CARTRIDGE_BOTTOM").is_none() {
			let mut stdin = tokio::io::stdin();
			let mut buffer = [0u8; 64];
			while stdin.read(&mut buffer).await.unwrap_or(0) > 0 {}
		} else {
			futures::future::pending::<()>().await;
		}
	};
	tokio::select! {
		_ = gone => {}
		_ = stopped() => {}
	}
	drop(dependent);
}

/// Bind the node's needs to its dependency: a client on the dependency's
/// socket, every need of the document a remote over it. The dependency served
/// before it re-entered, but the door is only ever a spawn away — a short
/// retry, then a refusal that names the link. The link dies with the
/// socket: a dependency that goes away fails every call still in flight and
/// the stdin EOF takes the node along.
async fn bind_dependency(
	host: &std::sync::Arc<Host>,
	root: &Path,
	dep: &str,
	needs: &[String],
) -> Result<(), String> {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
	let path = cartridge::socket::node_path(root, dep);
	let mut stream = None;
	for _ in 0..cartridge::settings::host().daemon_wait_attempts {
		match tokio::net::UnixStream::connect(&path).await {
			Ok(s) => {
				stream = Some(s);
				break;
			}
			Err(_) => tokio::time::sleep(cartridge::settings::host().daemon_wait()).await,
		}
	}
	let stream = stream.ok_or_else(|| {
		format!(
			"the dependency `{dep}` serves no socket at {}",
			path.display()
		)
	})?;
	let (read, mut write) = stream.into_split();
	let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<serde_json::Value>>();
	let link = cartridge::cartridge::Link::new(tx, "the dependency is gone");
	// Every line the dependency writes is a reply to a call this node made;
	// the socket protocol and the wire's reply envelope are the same shape,
	// so the link decodes both ends of the conversation itself.
	tokio::spawn({
		let link = link.clone();
		async move {
			let mut lines = tokio::io::BufReader::new(read).lines();
			while let Ok(Some(line)) = lines.next_line().await {
				if let Ok(m) = serde_json::from_str::<serde_json::Value>(&line) {
					link.accept(&m);
				}
			}
			link.shutdown();
		}
	});
	tokio::spawn(async move {
		while let Some(Some(m)) = rx.recv().await {
			let line = format!("{m}\n");
			if write.write_all(line.as_bytes()).await.is_err() {
				break;
			}
			if write.flush().await.is_err() {
				break;
			}
		}
	});
	for key in needs {
		host.bind_dependency(
			key,
			cartridge::cartridge::Remote::over(link.clone(), key.to_owned()),
		);
	}
	Ok(())
}

/// The environment every node of the tree carries: its identity in the
/// ledger, what is still to come after it, and how to re-enter the binary.
/// The chain rides in the environment, not in a coordinator: the ask
/// resolved it, and each node hands its remainder to its dependent.
fn node_env(command: &mut std::process::Command, node: &str, rest: &[String], dir: &Path) {
	command
		.env("CARTRIDGE_NODE", node)
		.env(
			"CARTRIDGE_CHAIN",
			serde_json::to_string(rest).expect("node paths serialize"),
		)
		.env("CARTRIDGE_CARTRIDGE", exe())
		.env("CARTRIDGE_ROOT", dir);
}

/// Start one node of the chain, detached. The invocation's job is to resolve
/// and hand off, and nothing sits above the tree owning it: the ask returns
/// while the tree keeps running, and the node's own death is what the rest
/// of the tree watches for.
fn launch_node(
	node: &cartridge::ledger::Installed,
	rest: &[&cartridge::ledger::Installed],
	dir: &Path,
) {
	let paths: Vec<String> = rest.iter().map(|e| e.path.clone()).collect();
	let mut command = match node_route(node) {
		Ok(Route::Program(program)) => tokio::process::Command::new(program),
		// The hosted node's program is this binary itself, in the node mode
		// the re-entry execs into. Nothing else differs: detached, named,
		// holding the env the re-entry reads.
		Ok(Route::Hosted) => {
			let mut command = tokio::process::Command::new(exe());
			command.arg("node");
			command
		}
		Err(e) => {
			eprintln!("{e}");
			std::process::exit(1);
		}
	};
	command.current_dir(&node.dir);
	node_env(command.as_std_mut(), &node.path, &paths, dir);
	// The far end was launched by the ask, not by a dependency: nothing
	// above it owns it, so it has no pipe to watch and nothing to cascade
	// to it. Everything above it is launched by the node below. Its stdio
	// is detached too — a node holding the asker's stdout would hold the
	// ask open forever.
	command
		.env("CARTRIDGE_BOTTOM", "1")
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null());
	match command.spawn() {
		Ok(child) => {
			println!("{}", child.id().unwrap_or(0));
			let _ = child;
		}
		Err(e) => {
			eprintln!("`{}`: {e}", node.path);
			std::process::exit(1);
		}
	}
}

/// Newline-delimited JSON-RPC between an MCP client and the `mcp` service:
/// every line of stdin is one message, every non-null reply one line of stdout.
/// One task per message, so a cancellation notification is read and acted on
/// while the call it cancels is still running. Diagnostics stay on stderr;
/// stdout carries the protocol and nothing else.
async fn stdio(host: std::sync::Arc<Host>) -> Result<Value, String> {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
	let (replies, mut pending) =
		tokio::sync::mpsc::channel::<String>(cartridge::settings::host().mcp_reply_queue);
	let writer = tokio::spawn(async move {
		let mut out = tokio::io::stdout();
		while let Some(line) = pending.recv().await {
			if out.write_all(line.as_bytes()).await.is_err() || out.flush().await.is_err() {
				break;
			}
		}
	});
	let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
	let mut serving = tokio::task::JoinSet::new();
	while let Ok(Some(line)) = lines.next_line().await {
		if line.trim().is_empty() {
			continue;
		}
		let (host, replies) = (host.clone(), replies.clone());
		serving.spawn(async move {
			let result = host
				.call("mcp", json!({ "op": "message", "line": line }))
				.await;
			if let Some(reply) = mcp_bridge_reply(&line, result) {
				let _ = replies.send(format!("{reply}\n")).await;
			}
		});
	}
	while serving.join_next().await.is_some() {}
	drop(replies);
	let _ = writer.await;
	Ok(Value::Null)
}

/// Bridge failures still owe a request its correlated protocol response.
/// Notifications and client responses never receive an error response.
fn mcp_bridge_reply(line: &str, result: Result<Value, String>) -> Option<Value> {
	match result {
		Ok(reply) => (!reply.is_null()).then_some(reply),
		Err(error) => {
			eprintln!("mcp: {error}");
			let message: Value = match serde_json::from_str(line) {
				Ok(message) => message,
				Err(_) => {
					return Some(
						json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}),
					)
				}
			};
			message.get("method")?.as_str()?;
			let id = message.get("id").filter(|id| !id.is_null())?;
			Some(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":error}}))
		}
	}
}

#[cfg(test)]
#[path = "../.cartridge/tests/unit/stdio.rs"]
mod stdio_tests;

/// CLI service arguments use JSON.
fn json_arg(s: String) -> Value {
	serde_json::from_str(&s).unwrap_or_else(|error| {
		eprintln!("invalid JSON argument: {error}");
		std::process::exit(2);
	})
}

/// A request naming a trace gets it back on the reply, so a probe report can
/// cite the id of one specific call.
async fn ask(profile: &std::path::Path, request: Value, answer: &str) {
	let trace = request.get("trace").cloned();
	let mut client = connect(profile).await;
	client.send(request).await.expect("send");
	while let Some(mut m) = client.next().await {
		if m.get(answer).is_some() || m.get("error").is_some_and(|e| e.get("cartridge").is_some()) {
			if let (Some(trace), Some(o)) = (&trace, m.as_object_mut()) {
				o.insert("trace".into(), trace.clone());
			}
			println!("{m}");
			return;
		}
	}
	eprintln!(
		"no {answer} from {}: the host closed the connection",
		profile.display()
	);
	std::process::exit(1);
}

fn deps(all: &[CartridgeInfo], cartridge: &CartridgeInfo, depth: usize, stack: &mut Vec<String>) {
	let pad = "  ".repeat(depth);
	for key in &cartridge.inject {
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
fn list(cartridges: &[CartridgeInfo]) -> usize {
	for p in cartridges {
		let mut line = format!("{}  {}", p.entry.id, p.entry.path);
		if p.entry.disabled {
			line.push_str("  (disabled)");
		}
		if !p.provide.is_empty() {
			line.push_str(&format!("  provides {}", p.provide.join(", ")));
		}
		if !p.export.is_empty() {
			line.push_str(&format!("  exports {}", p.export.join(", ")));
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

/// The host's own settings, settled against its declarations, for the three
/// renderings below. A configuration the declarations refuse does not silently
/// become the defaults here: the reason is said once, and then the defaults
/// stand — a limit that will not parse must not take the listing of limits down
/// with it.
fn host_settled(profile: &Path) -> Value {
	use cartridge::settings;
	let configured = settings::layers(profile)
		.unwrap_or_else(|e| {
			eprintln!("cartridge: settings: {e}");
			json!({})
		})
		.get("host")
		.cloned()
		.unwrap_or_else(|| json!({}));
	settings::apply(settings::host_specs(), configured, "host").unwrap_or_else(|e| {
		eprintln!("cartridge: settings: {e}; showing declared defaults");
		settings::defaults(settings::host_specs())
	})
}

/// Every tunable value, in one table: the host's own first, then each
/// cartridge's, each key with its type, what it is set to, and where that came
/// from. This is the listing the system's own documentation points at, so a
/// value that cannot be found here is a value that was never a setting.
///
/// Answers how many problems it found: a cartridge configured with keys it
/// never declared is one per cartridge, so finishing the migration is a
/// non-zero exit going to zero rather than a memory of which ones were done.
fn settings_table(profile: &Path, entries: &[loader::SettingsInfo], what: Option<&str>) -> usize {
	use cartridge::settings;
	let wanted = |section: &str, key: &str| match what {
		None => true,
		Some(w) => {
			let dotted = format!("{section}.{key}");
			section == w || dotted == w || dotted.starts_with(&format!("{w}."))
		}
	};
	let mut rows: Vec<(String, String, String, String, String)> = Vec::new();
	let mut push = |section: &str, key: &str, spec: Option<&settings::Spec>, value: &Value| {
		if !wanted(section, key) {
			return;
		}
		let dotted = format!("{section}.{key}");
		let declared = spec.map_or(Value::Null, |s| s.default.clone());
		let source = settings::source(profile, &dotted, value, &declared);
		let kind = spec.map_or("undeclared".to_owned(), |s| {
			s.describe()["type"].as_str().unwrap_or("?").to_owned()
		});
		let doc = spec.and_then(|s| s.doc.clone()).unwrap_or_default();
		rows.push((
			dotted,
			kind,
			serde_json::to_string(value).unwrap_or_default(),
			source.to_owned(),
			doc,
		));
	};
	let host = host_settled(profile);
	for (key, spec) in settings::host_specs() {
		let value = settings::get(&host, key).cloned().unwrap_or(Value::Null);
		push("host", key, Some(spec), &value);
	}
	for entry in entries {
		for (key, spec) in &entry.specs {
			let value = settings::get(&entry.settled, key)
				.cloned()
				.unwrap_or(Value::Null);
			push(&entry.id, key, Some(spec), &value);
		}
		// A configured key with no declaration is listed too, and marked. It is
		// working configuration — dropping it from the listing would hide the
		// one thing this listing exists to find.
		for key in &entry.undeclared {
			let value = settings::get(&entry.settled, key)
				.cloned()
				.unwrap_or(Value::Null);
			push(&entry.id, key, None, &value);
		}
	}
	// One wide value — a whole table of per-tool rules is a common one — must
	// not push every other column off the terminal, so the value column is
	// elided past a readable width. `--json` is the un-elided answer.
	const VALUE_WIDTH: usize = 44;
	let width = |n: usize| rows.iter().map(|r| field(r, n).len()).max().unwrap_or(0);
	let (w0, w1, w3) = (width(0), width(1), width(3));
	let w2 = width(2).min(VALUE_WIDTH);
	for row in &rows {
		let doc = &row.4;
		let value = match row.2.chars().count() > VALUE_WIDTH {
			true => format!(
				"{}…",
				row.2.chars().take(VALUE_WIDTH - 1).collect::<String>()
			),
			false => row.2.clone(),
		};
		let line = format!(
			"{:w0$}  {:w1$}  {:>w2$}  {:w3$}",
			row.0, row.1, value, row.3
		);
		match doc.is_empty() {
			true => println!("{line}"),
			false => println!("{line}  {doc}"),
		}
	}
	// Only what was asked about is counted: narrowing the listing to one
	// cartridge asks about that cartridge, and answering for the rest of the
	// profile would make a clean one look dirty.
	entries
		.iter()
		.filter(|e| e.undeclared.iter().any(|key| wanted(&e.id, key)))
		.count()
}

fn field(row: &(String, String, String, String, String), n: usize) -> &str {
	match n {
		0 => &row.0,
		1 => &row.1,
		2 => &row.2,
		_ => &row.3,
	}
}

/// The listing as data: declarations, settled values and the file each came
/// from, for anything reading this surface rather than looking at it.
fn settings_json(profile: &Path, entries: &[loader::SettingsInfo]) -> String {
	use cartridge::settings;
	let describe =
		|section: &str, specs: &settings::Specs, settled: &Value, undeclared: &[String]| {
			let keys: serde_json::Map<String, Value> = specs
				.iter()
				.map(|(key, spec)| {
					let mut out = spec.describe();
					let map = out.as_object_mut().expect("object");
					map.insert(
						"value".into(),
						settings::get(settled, key).cloned().unwrap_or(Value::Null),
					);
					map.insert(
						"source".into(),
						json!(settings::source(
							profile,
							&format!("{section}.{key}"),
							settings::get(settled, key).unwrap_or(&Value::Null),
							&spec.default,
						)),
					);
					(key.clone(), out)
				})
				.collect();
			json!({"keys": keys, "undeclared": undeclared})
		};
	let host_settled = host_settled(profile);
	let mut out = serde_json::Map::new();
	out.insert(
		"host".into(),
		describe("host", settings::host_specs(), &host_settled, &[]),
	);
	for entry in entries {
		out.insert(
			entry.id.clone(),
			describe(&entry.id, &entry.specs, &entry.settled, &entry.undeclared),
		);
	}
	serde_json::to_string_pretty(&Value::Object(out)).unwrap_or_default()
}

/// The same surface as a `config.lua`: every key, its documentation above it,
/// and its current value. Saving this as `~/.cartridge/config.lua` changes
/// nothing and leaves every knob in reach, which is the point — a person
/// tuning a system should not have to discover the key's name first.
fn settings_template(profile: &Path, entries: &[loader::SettingsInfo]) {
	use cartridge::settings;
	println!("-- Every setting this profile has, at its current value.");
	println!("-- Save as ~/.cartridge/config.lua for this machine, or as");
	println!("-- .cartridge/config.lua for this project alone. Delete what you");
	println!("-- do not want to pin: an absent key keeps its declared default.");
	println!("return {{");
	let host = host_settled(profile);
	section("host", settings::host_specs(), &host);
	for entry in entries {
		if entry.specs.is_empty() {
			continue;
		}
		section(&entry.id, &entry.specs, &entry.settled);
	}
	println!("}}");
}

/// One cartridge's table, written as the nested tables its dotted keys mean.
/// `ship.remote` is `ship = { remote = ... }` here and not a key with a dot in
/// its name, which is a different thing and would configure nothing.
fn section(id: &str, specs: &cartridge::settings::Specs, settled: &Value) {
	println!("\t{} = {{", lua_key(id));
	table(settled, specs, "", 2);
	println!("\t}},");
}

fn table(value: &Value, specs: &cartridge::settings::Specs, prefix: &str, depth: usize) {
	let Some(map) = value.as_object() else {
		return;
	};
	let pad = "\t".repeat(depth);
	for (key, value) in map {
		let dotted = match prefix.is_empty() {
			true => key.clone(),
			false => format!("{prefix}.{key}"),
		};
		if let Some(doc) = specs.get(&dotted).and_then(|s| s.doc.as_deref()) {
			println!("{pad}-- {doc}");
		}
		// A table with declarations under it is written out key by key, so each
		// leaf keeps its own line and its own comment. One the declaration does
		// not reach is written inline: its shape is the user's, not ours.
		let inside = specs.keys().any(|k| k.starts_with(&format!("{dotted}.")));
		match value.is_object() && inside {
			true => {
				println!("{pad}{} = {{", lua_key(key));
				table(value, specs, &dotted, depth + 1);
				println!("{pad}}},");
			}
			false => println!("{pad}{} = {},", lua_key(key), lua_value(value)),
		}
	}
}

/// A name Lua can take bare, or the bracketed string form for one it cannot —
/// `live-record` is a key, not an identifier.
fn lua_key(key: &str) -> String {
	let bare = !key.is_empty()
		&& !key.starts_with(|c: char| c.is_ascii_digit())
		&& key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
	match bare {
		true => key.to_owned(),
		false => format!("[{key:?}]"),
	}
}

fn lua_value(value: &Value) -> String {
	match value {
		Value::Null => "nil".to_owned(),
		Value::Bool(b) => b.to_string(),
		Value::Number(n) => n.to_string(),
		Value::String(s) => format!("{s:?}"),
		Value::Array(items) => format!(
			"{{ {} }}",
			items.iter().map(lua_value).collect::<Vec<_>>().join(", ")
		),
		Value::Object(map) => format!(
			"{{ {} }}",
			map.iter()
				.map(|(k, v)| format!("{} = {}", lua_key(k), lua_value(v)))
				.collect::<Vec<_>>()
				.join(", ")
		),
	}
}

/// One line per installed cartridge, in path order, with each of its needs
/// under it bound to the path that provides it — `?` where nothing in scope
/// does, `ambiguous` naming every offer where one scope offers it twice.
/// Answers how many **problems** the listing found, on the same terms as
/// [`list`]: an unreadable document is listed and counted, never silently
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
		if !e.export.is_empty() {
			line.push_str(&format!("  exports {}", e.export.join(", ")));
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

/// A path pinned to the directory this process started in, lexically: the
/// answer must not change once the working directory moves to the project root,
/// and a directory that does not exist yet still has an address.
/// One cartridge as the manual sees it: the declarations off its document and
/// the page it ships. The page is `.cartridge/help.md` in the cartridge's own
/// directory, so what `help` prints for a cartridge is what that cartridge
/// brought, and nothing here describes one cartridge from inside another.
struct HelpEntry {
	id: String,
	path: String,
	enabled: bool,
	/// The cartridge's real directory, symlinks resolved: the base every link
	/// on its page is relative to, printed so a reader can follow them.
	dir: PathBuf,
	document: Result<loader::Cartridge, String>,
	inject: Vec<String>,
	page: Option<String>,
}

/// The manual: the host's own page beside the profile, and one entry per
/// cartridge the ledger knows, enabled or not. Nothing is cached and nothing
/// is authored here: reading it again after a cartridge is installed or
/// removed is the whole update.
struct Manual {
	host_page: Option<String>,
	host_dir: PathBuf,
	cartridges: Vec<HelpEntry>,
}

const PAGE: &str = "help.md";

impl Manual {
	fn read(dir: &Path, profile: &Path, cartridges: &[CartridgeInfo]) -> Self {
		let host_dir = std::fs::canonicalize(profile).unwrap_or_else(|_| profile.to_path_buf());
		let cartridges = cartridges
			.iter()
			.map(|c| {
				let manifest = c.entry.file(dir);
				let home = manifest.parent().map(Path::to_path_buf).unwrap_or_default();
				let page = std::fs::read_to_string(home.join(".cartridge").join(PAGE)).ok();
				HelpEntry {
					id: c.entry.id.clone(),
					path: c.entry.path.clone(),
					enabled: !c.entry.disabled,
					dir: std::fs::canonicalize(&home).unwrap_or(home),
					document: loader::Cartridge::document(&manifest),
					inject: c.inject.clone(),
					page,
				}
			})
			.collect();
		Self {
			host_page: std::fs::read_to_string(profile.join(PAGE)).ok(),
			host_dir,
			cartridges,
		}
	}

	fn description(entry: &HelpEntry) -> String {
		match &entry.document {
			Ok(doc) => doc.description.clone().unwrap_or_default(),
			Err(e) => format!("error: {e}"),
		}
	}

	/// The spine: every enabled cartridge with its one line, then what is
	/// installed and not enabled, then the ways to go deeper. Answers how many
	/// enabled cartridges have no page or no readable document — the sweep this
	/// listing exists to finish, said in the exit status like `settings` does.
	fn overview(&self) -> usize {
		let enabled: Vec<_> = self.cartridges.iter().filter(|c| c.enabled).collect();
		let parked: Vec<_> = self.cartridges.iter().filter(|c| !c.enabled).collect();
		let width = enabled
			.iter()
			.chain(&parked)
			.map(|c| c.id.len())
			.max()
			.unwrap_or(4)
			.max(4);
		println!(
			"cartridge help: {} cartridges enabled, {} installed and not enabled\n",
			enabled.len(),
			parked.len()
		);
		println!(
			"  {:width$}  the runtime itself: what a cartridge is, how one is written, how this composition is read{}",
			"host",
			if self.host_page.is_some() { "" } else { "  (no page)" }
		);
		let mut problems = usize::from(self.host_page.is_none());
		for c in &enabled {
			let mut line = format!("  {:width$}  {}", c.id, Self::description(c));
			if c.page.is_none() {
				line.push_str("  (no page)");
			}
			if c.document.is_err() || c.page.is_none() {
				problems += 1;
			}
			println!("{line}");
		}
		if !parked.is_empty() {
			println!("\ninstalled, not enabled:");
			for c in &parked {
				println!("  {:width$}  {}", c.id, Self::description(c));
			}
		}
		println!(
			"\ngo deeper:\n  cartridge help <id>      one cartridge: its declarations, then the page it ships\n  cartridge help <word>    search every page and description\n  cartridge help host      the runtime's own page\n  cartridge help --json    the whole manual as one JSON document\n  cartridge settings       every tunable value and the file that settled it\n  cartridge list           what each cartridge needs, resolved to its provider"
		);
		if problems > 0 {
			eprintln!(
				"\n{problems} enabled cartridge(s) ship no .cartridge/{PAGE} or no readable document: a cartridge documents itself, so the fix is a page in that cartridge"
			);
		}
		problems
	}

	fn host_page(&self) -> usize {
		println!("# host  ({})\n", self.host_dir.display());
		println!("The runtime that composes the cartridges. Its own tunable values are under `cartridge settings host`; its composition is `cartridge list`.");
		match &self.host_page {
			Some(page) => {
				println!(
					"\npage: {}  (links are relative to {})\n---\n{}",
					self.host_dir.join(PAGE).display(),
					self.host_dir.display(),
					page.trim_end()
				);
				0
			}
			None => {
				eprintln!(
					"\nno page: {} does not exist",
					self.host_dir.join(PAGE).display()
				);
				1
			}
		}
	}

	/// One cartridge in full: what its document declares, then its page
	/// verbatim. The declarations are read off `cartridge.json` here rather
	/// than restated on the page, so the page cannot go stale on them.
	fn entry(&self, id: &str) -> usize {
		let Some(c) = self.cartridges.iter().find(|c| c.id == id) else {
			return 1;
		};
		println!(
			"# {}  ({} -> {}){}\n",
			c.id,
			c.path,
			c.dir.display(),
			if c.enabled {
				""
			} else {
				"  [installed, not enabled]"
			}
		);
		let mut problems = 0;
		match &c.document {
			Ok(doc) => {
				if let Some(d) = &doc.description {
					println!("{d}\n");
				}
				let list = |label: &str, keys: &[String]| {
					if !keys.is_empty() {
						println!("{label}: {}", keys.join(", "));
					}
				};
				list("provides", &doc.provide);
				list("needs", &doc.needs);
				list("injected", &c.inject);
				list("exports", &doc.export);
				if !doc.settings.is_empty() {
					println!("settings:  (cartridge settings {})", c.id);
					for (key, spec) in &doc.settings {
						println!("  {key:24} {}", spec.doc.as_deref().unwrap_or(""));
					}
				}
				if !doc.commands.is_empty() {
					println!("commands:  (run from {})", c.dir.display());
					for (label, cmd) in &doc.commands {
						let mut line = format!("  {label:8} {}", cmd.argv.join(" "));
						if cmd.cwd != "." {
							line.push_str(&format!("  (cwd {})", cmd.cwd));
						}
						if let Some(d) = &cmd.description {
							line.push_str(&format!("  # {d}"));
						}
						println!("{line}");
					}
				}
			}
			Err(e) => {
				println!("document: error: {e}");
				problems += 1;
			}
		}
		match &c.page {
			Some(page) => println!(
				"\npage: {}  (links are relative to {})\n---\n{}",
				c.dir.join(".cartridge").join(PAGE).display(),
				c.dir.display(),
				page.trim_end()
			),
			None => {
				eprintln!(
					"\nno page: {} does not exist. A cartridge documents itself; write that file",
					c.dir.join(".cartridge").join(PAGE).display()
				);
				problems += 1;
			}
		}
		problems
	}

	/// A word across the manual: ids, descriptions, keys, settings
	/// documentation and every line of every page, case-insensitively. One
	/// line per hit, prefixed by the cartridge it was found in, so a reader
	/// knows which `cartridge help <id>` to open next.
	fn search(&self, word: &str) -> usize {
		let needle = word.to_lowercase();
		let hit = |text: &str| text.to_lowercase().contains(&needle);
		let mut hits = 0;
		let mut show = |id: &str, line: &str| {
			println!("{id}:  {}", line.trim());
			hits += 1;
		};
		if let Some(page) = &self.host_page {
			for line in page.lines().filter(|l| hit(l)) {
				show("host", line);
			}
		}
		for c in &self.cartridges {
			if hit(&c.id) {
				show(&c.id, &format!("{}  {}", c.id, Self::description(c)));
			} else if hit(&Self::description(c)) {
				show(&c.id, &Self::description(c));
			}
			if let Ok(doc) = &c.document {
				for key in doc.provide.iter().chain(&doc.needs).filter(|k| hit(k)) {
					show(&c.id, &format!("key {key}"));
				}
				for (key, spec) in &doc.settings {
					let doc_line = spec.doc.as_deref().unwrap_or("");
					if hit(key) || hit(doc_line) {
						show(&c.id, &format!("setting {}.{key}  {doc_line}", c.id));
					}
				}
			}
			if let Some(page) = &c.page {
				for line in page.lines().filter(|l| hit(l)) {
					show(&c.id, line);
				}
			}
		}
		if hits == 0 {
			eprintln!(
				"nothing in the manual matches `{word}`; `cartridge help` lists what is composed"
			);
			return 1;
		}
		0
	}

	/// The whole manual as one document, for anything reading it rather than
	/// looking at it. Every optional field is materialised (`null`, `[]`, `{}`),
	/// never omitted, so a consumer does not have to tell absent from empty.
	fn json(&self) -> String {
		let cartridges: Vec<Value> = self
			.cartridges
			.iter()
			.map(|c| {
				let (description, provide, needs, export, settings, commands, error) =
					match &c.document {
						Ok(doc) => (
							json!(doc.description),
							json!(doc.provide),
							json!(doc.needs),
							json!(doc.export),
							doc.settings
								.iter()
								.map(|(k, s)| (k.clone(), json!(s.doc)))
								.collect::<serde_json::Map<_, _>>(),
							doc.commands
								.iter()
								.map(|(k, cmd)| {
									(
										k.clone(),
										json!({"argv": cmd.argv, "cwd": cmd.cwd, "description": cmd.description}),
									)
								})
								.collect::<serde_json::Map<_, _>>(),
							Value::Null,
						),
						Err(e) => (
							Value::Null,
							json!([]),
							json!([]),
							json!([]),
							Default::default(),
							Default::default(),
							json!(e),
						),
					};
				json!({
					"id": c.id,
					"path": c.path,
					"dir": c.dir,
					"enabled": c.enabled,
					"description": description,
					"provide": provide,
					"needs": needs,
					"injected": c.inject,
					"export": export,
					"settings": settings,
					"commands": commands,
					"page": c.page,
					"error": error,
				})
			})
			.collect();
		json!({
			"host": { "dir": self.host_dir, "page": self.host_page },
			"cartridges": cartridges,
		})
		.to_string()
	}
}

fn absolute(path: &Path) -> PathBuf {
	std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The two ways a process is asked to stop: an interrupt from the terminal it
/// runs in, and a terminate from whatever supervises it. Either one is a
/// request, and a host that honours it gets to put its socket away.
async fn stopped() {
	use tokio::signal::unix::{signal, SignalKind};
	let terminate = async {
		match signal(SignalKind::terminate()) {
			Ok(mut term) => {
				term.recv().await;
			}
			Err(_) => futures::future::pending().await,
		}
	};
	tokio::select! {
		_ = tokio::signal::ctrl_c() => {}
		_ = terminate => {}
	}
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	if cli.yolo && !matches!(&cli.command, Command::Run { .. } | Command::Daemon) {
		eprintln!("--yolo requires run or daemon; it cannot change an existing daemon");
		std::process::exit(2);
	}
	// `--dir` is the last path read against the directory this process was
	// started in; everything after is read against the project. A runtime is
	// bound to its project and dies with the terminal that started it, so the
	// project is decided once, here, before anything is loaded — and a command
	// typed in a subdirectory joins the runtime already serving that project
	// instead of starting a second one beside it.
	let dir = absolute(&cli.dir.unwrap_or_else(loader::builtin));
	let root = loader::root();
	if let Err(e) = std::env::set_current_dir(&root) {
		eprintln!("{}: {e}", root.display());
		std::process::exit(1);
	}
	// One user profile, one composition: `mcp`, `launch` and `run` differ by the
	// entry point they call into this host, not by the cartridges it loads.
	let profile = loader::profile();
	// Settle the host's own settings against the profile that was just
	// resolved, before anything reads one. Everything downstream — including an
	// SDK child that has only a working directory — then reads this one answer.
	cartridge::settings::settle(&profile);
	match cli.command {
		Command::Run { key, args } => {
			// Foreground process cartridges receive terminal interrupts directly.
			let signals = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
			let host = Host::with_yolo(Runtime::new(), &dir, &profile, cli.yolo);
			// The live host also answers on its socket, so `cartridge call`, `status`
			// and `tail` from the wrapped shell reach *this* process instead of
			// loading a second copy of the profile. A second `run` on the same
			// profile keeps working; only its socket is refused.
			let served = tokio::spawn({
				let (host, path) = (host.clone(), socket::path(&profile));
				async move {
					match socket::serve(host, &path).await {
						// Another run already answers for this profile; that one keeps it.
						Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {}
						Err(e) => eprintln!("socket {}: {e}", path.display()),
						Ok(()) => {}
					}
				}
			});
			let result = host.run(&key, json_arg(args)).await;
			served.abort();
			signals.abort();
			match result {
				Ok(value) if !value.is_null() => println!("{value}"),
				Ok(_) => {}
				Err(error) => {
					eprintln!("{error}");
					cartridge::trace::diagnostic("cartridge", error, Value::Null);
					std::process::exit(1);
				}
			}
		}
		Command::Launch { agent, model, args } => {
			// The proxy listener needs a key; the launched agent gets the same one.
			if std::env::var("CARTRIDGE_PROXY_KEY").map_or(true, |k| k.is_empty()) {
				std::env::set_var(
					"CARTRIDGE_PROXY_KEY",
					random_hex(cartridge::settings::host().proxy_key_bytes),
				);
			}
			let signals = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
			let host = Host::new(Runtime::new(), &dir, &profile);
			let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args });
			let result = host.run_then("proxy", request, spawn).await;
			signals.abort();
			match result {
				Ok(status) => std::process::exit(status.as_i64().unwrap_or(1) as i32),
				Err(error) => {
					eprintln!("{error}");
					cartridge::trace::diagnostic("cartridge", error, Value::Null);
					std::process::exit(1);
				}
			}
		}
		Command::Mcp => {
			// The client owns this process's lifetime: the pump ends at end of
			// input and the profile is disposed on the way out.
			let host = Host::new(Runtime::new(), &dir, &profile);
			let serve = {
				let host = host.clone();
				|_| async move { stdio(host).await }
			};
			if let Err(error) = host.run_then("mcp", json!({ "op": "ready" }), serve).await {
				eprintln!("{error}");
				std::process::exit(1);
			}
		}
		Command::Daemon => {
			let host = Host::with_yolo(Runtime::new(), &dir, &profile, cli.yolo);
			if let Err(e) = host.reconcile().await {
				cartridge::trace::diagnostic("init.lua", e, Value::Null);
			}
			if let Err(e) = host.watch() {
				cartridge::trace::diagnostic("watch", e, Value::Null);
			}
			let path = socket::path(&profile);
			cartridge::trace::diagnostic(
				"cartridge",
				"serving",
				json!({ "dir": dir.display().to_string(), "profile": profile.display().to_string(), "socket": path.display().to_string() }),
			);
			// A daemon outlives every other command, so it is the one that
			// keeps the socket directory: the collecting runs beside the
			// serving, off the path of anything waiting on the door to open.
			tokio::spawn(cartridge::socket::keep_swept());
			// The stop is caught rather than taken: the socket is unlinked by a
			// guard the serving holds, and a guard only runs on the way out of
			// a future that was allowed to end. Killed outright there is no way
			// out, and the next sweep is what collects the entry.
			tokio::select! {
				served = socket::serve(host, &path) => {
					if let Err(e) = served {
						cartridge::trace::diagnostic("cartridge", e, Value::Null);
						std::process::exit(1);
					}
				}
				_ = stopped() => {}
			}
		}
		Command::Send { name, data } => {
			connect(&profile)
				.await
				.send(json!({ "emit": name, "data": json_arg(data) }))
				.await
				.expect("send");
		}
		Command::Publish { channel, data } => {
			connect(&profile)
				.await
				.send(json!({ "publish": channel, "data": json_arg(data) }))
				.await
				.expect("send");
		}
		Command::Follow { channel } => {
			let mut client = connect(&profile).await;
			client
				.send(json!({ "subscribe": channel }))
				.await
				.expect("send");
			while let Some(m) = client.next().await {
				if m.get("channel").is_some() || m.get("error").is_some() {
					println!("{m}");
				}
			}
		}
		Command::Tail => {
			let mut client = connect(&profile).await;
			while let Some(m) = client.next().await {
				println!("{m}");
			}
		}
		Command::Call { key, args, trace } => {
			let trace = trace.unwrap_or_else(|| cartridge::trace::mint().to_string());
			ask(
				&profile,
				json!({ "call": key, "args": json_arg(args), "id": 1, "trace": trace }),
				"reply",
			)
			.await;
		}
		Command::Reload => ask(&profile, json!({ "reload": true }), "reloaded").await,
		Command::Status => ask(&profile, json!({ "status": true }), "status").await,
		Command::Debug { state } => ask(&profile, json!({ "debug": state }), "debug").await,
		Command::Socket => println!("{}", socket::path(&profile).display()),
		Command::Sweep => println!("{} swept", cartridge::socket::sweep().await),
		Command::Verify { cartridge } => {
			let host = Host::new(Runtime::new(), &dir, &profile);
			let run = match &cartridge {
				Some(one) => host.verify_one(one).await,
				None => host.verify().await,
			};
			match run {
				Ok((ran, failures)) if failures.is_empty() => println!("{ran} contracts passed"),
				Ok((_, failures)) => {
					for failure in failures {
						eprintln!("{failure}");
					}
					std::process::exit(1);
				}
				Err(error) => {
					eprintln!("{error}");
					std::process::exit(1);
				}
			}
		}
		Command::Ledger => {
			let ledger = cartridge::ledger::Ledger::scan(&dir);
			if ledger_lines(&ledger) > 0 {
				std::process::exit(1);
			}
		}
		Command::Up { key } => {
			let ledger = cartridge::ledger::Ledger::scan(&dir);
			// One ask, one resolution: the walk runs to the far end here, and
			// every link after it is a re-entry that re-reads the ledger. A
			// refusal is a refusal of the launch: nothing spawns, and the exit
			// says so.
			match cartridge::resolver::chain(&ledger, "", &key) {
				Err(refusal) => {
					eprintln!("{refusal}");
					std::process::exit(1);
				}
				Ok(chain) => {
					// Every node of the chain is checked before the first one
					// spawns: a refusal belongs to the ask, not to a link that
					// already came up.
					for node in &chain {
						if let Err(e) = node_route(node) {
							eprintln!("{e}");
							std::process::exit(1);
						}
					}
					let (bottom, rest) = chain
						.split_first()
						.expect("a resolved chain is never empty");
					launch_node(bottom, rest, &dir);
					// The tree answers at the top: the socket the named tool's
					// node serves is where its keys are called, derived by
					// anyone from the root and the node's ledger path.
					let top = chain.last().expect("a resolved chain is never empty");
					println!(
						"{}",
						json!({ "up": key, "nodes": chain.iter().map(|n| n.path.clone()).collect::<Vec<_>>(), "socket": cartridge::socket::node_path(&dir, &top.path) })
					);
				}
			}
		}
		Command::Enter { node, rest } => {
			let ledger = cartridge::ledger::Ledger::scan(&dir);
			// The step re-reads the ledger: a node that is no longer installed
			// has no launch, however recently the chain held it.
			let Some(entry) = ledger.get(&node) else {
				eprintln!("`{node}` is no longer installed: nothing launches");
				std::process::exit(1);
			};
			// A document that declares a binary execs into it; one that does
			// not is hosted, and the host's own node mode is the program.
			let mut command = match node_route(entry) {
				Ok(Route::Program(program)) => std::process::Command::new(program),
				Ok(Route::Hosted) => {
					let mut command = std::process::Command::new(exe());
					command.arg("node");
					command
				}
				Err(e) => {
					eprintln!("{e}");
					std::process::exit(1);
				}
			};
			use std::os::unix::process::CommandExt;
			command
				.env("CARTRIDGE_NODE", &node)
				.env("CARTRIDGE_CHAIN", rest)
				.env("CARTRIDGE_CARTRIDGE", exe())
				.env("CARTRIDGE_ROOT", &dir)
				// The far end was the only node nothing launched; a node
				// re-entered into has a dependency above it and loses the flag.
				.env_remove("CARTRIDGE_BOTTOM")
				// The node's stdin is the dependency's pipe, inherited: when
				// the dependency that launched it goes away, the pipe closes
				// and EOF is the signal to take the rest of the tree along.
				.current_dir(&entry.dir);
			let error = command.exec();
			eprintln!("{node}: {error}");
			std::process::exit(1);
		}
		Command::Node => hosted_node().await,
		Command::Settings {
			what,
			json: as_json,
			template,
		} => {
			let host = Host::new(Runtime::new(), &dir, &profile);
			let entries = match host.settings() {
				Ok(entries) => entries,
				Err(e) => {
					eprintln!("{}: {e}", profile.join("init.lua").display());
					std::process::exit(1);
				}
			};
			if template {
				settings_template(&profile, &entries);
				return;
			}
			if as_json {
				println!("{}", settings_json(&profile, &entries));
				return;
			}
			// A cartridge configured with keys it never declared is the one
			// thing this listing is for; saying so in the exit status is what
			// keeps the sweep finishable.
			if settings_table(&profile, &entries, what.as_deref()) > 0 {
				std::process::exit(1);
			}
		}
		Command::Help {
			what,
			json: as_json,
		} => {
			let host = Host::new(Runtime::new(), &dir, &profile);
			let cartridges = match host.manifest() {
				Ok(cartridges) => cartridges,
				Err(e) => {
					eprintln!("{}: {e}", profile.join("init.lua").display());
					std::process::exit(1);
				}
			};
			let manual = Manual::read(&dir, &profile, &cartridges);
			if as_json {
				if what.is_some() {
					eprintln!("--json is the whole manual; it takes no query");
					std::process::exit(2);
				}
				println!("{}", manual.json());
				return;
			}
			let problems = match what.as_deref() {
				None => manual.overview(),
				Some("host") => manual.host_page(),
				Some(id) if manual.cartridges.iter().any(|c| c.id == id) => manual.entry(id),
				Some(word) => manual.search(word),
			};
			if problems > 0 {
				std::process::exit(1);
			}
		}
		Command::List => {
			// `--yolo` is rejected above for every command but run and daemon.
			let host = Host::new(Runtime::new(), &dir, &profile);
			match host.manifest() {
				Ok(cartridges) => {
					// A document that would not read is a failure of the listing, not a
					// footnote in it: the line is printed, and the exit says so.
					if list(&cartridges) > 0 {
						std::process::exit(1);
					}
				}
				Err(e) => {
					eprintln!("{}: {e}", profile.join("init.lua").display());
					std::process::exit(1);
				}
			}
		}
	}
}
