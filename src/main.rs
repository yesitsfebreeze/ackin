use std::path::{Path, PathBuf};
use std::process::Stdio;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use zirkle::loader::{self, CartridgeInfo};
use zirkle::lua::Host;
use zirkle::runtime::Runtime;
use zirkle::socket::{self, Client};

#[derive(Parser)]
#[command(
	name = "zirkle",
	about = "cartridges on a socket: run in the back, handle the events"
)]
struct Cli {
	/// Directory containing bundled cartridges (each with cartridge.json)
	#[arg(long, global = true)]
	dir: Option<PathBuf>,
	/// Profile name under `.zirkle/`, or an absolute profile directory
	/// (default `default`; `proxy` for launch)
	#[arg(long, global = true)]
	profile: Option<String>,
	/// Automatic execution: bypass tool policy and skip resolver provenance recording
	#[arg(long, global = true)]
	yolo: bool,
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	Daemon,
	/// Start the harness proxy and run an agent against it: `zirkle launch claude -- -p hi`
	Launch {
		agent: String,
		#[arg(long, default_value = "auto:code")]
		model: String,
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		args: Vec<String>,
	},
	/// Serve the profile's tools to an MCP client over this terminal's stdio:
	/// `claude mcp add zirkle -- zirkle mcp`
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
	Tail,
	/// Call a provided key with one JSON argument and print the reply
	Call {
		key: String,
		#[arg(default_value = "null")]
		args: String,
		/// Tag this call with a turn id; one is minted when absent
		#[arg(long)]
		turn: Option<String>,
	},
	Reload,
	Status,
	/// Enter, leave or report debug mode on the running host: `zirkle debug on`
	Debug {
		#[arg(default_value = "status", value_parser = ["on", "off", "status"])]
		state: String,
	},
	Socket,
	/// Cartridges of the profile and what each one needs, resolved to its provider
	List,
	/// Every cartridge installed under the cartridge root, and what each need binds to
	Ledger,
	/// Resolve a tool's chain off the ledger, to its far end, and start it
	/// there. The far end's program comes up and re-enters this binary for
	/// the next link, one re-entry per link, until the named tool is running
	/// at the top of a tree assembled out of whatever the ledger held at the
	/// moment of the ask
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
	/// Load the profile and run every contract its cartridges declare
	Verify,
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
/// the cartridge's own `bin/`, then beside the running zirkle, then `PATH`.
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
fn node_route(
	node: &zirkle::ledger::Installed,
) -> Result<Route, String> {
	let manifest = node.dir.join(zirkle::loader::MANIFEST);
	let document = zirkle::loader::Cartridge::document(&manifest)
		.map_err(|e| format!("`{}`: {e}", node.path))?;
	match document.binary {
		Some(binary) => zirkle::cartridge::executable(&binary, &node.dir)
			.map(Route::Program)
			.map_err(|e| format!("`{}`: {e}", node.path)),
		// The cartridge has no program of its own, so the host is it: the
		// entry is resolved now, not evaluated — a missing Lua file is a
		// refusal of the ask, an entry that fails to apply is the node's.
		None => zirkle::loader::Cartridge::read(&manifest)
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
/// The pid it writes under `ZIRKLE_NODES` is the probe's observation handle,
/// not part of the mechanism: a test reads it to name the process it is
/// asserting about, exactly as the program fixture does.
async fn hosted_node() {
	use tokio::io::AsyncReadExt;

	let node = std::env::var("ZIRKLE_NODE").unwrap_or_default();
	let chain: Vec<String> = serde_json::from_str(
		&std::env::var("ZIRKLE_CHAIN").unwrap_or_default(),
	)
	.unwrap_or_default();
	let (zirkle, root) = match (
		std::env::var("ZIRKLE_ZIRKLE"),
		std::env::var("ZIRKLE_ROOT"),
	) {
		(Ok(zirkle), Ok(root)) => (zirkle, root),
		_ => {
			eprintln!("node mode is entered, not asked: the ledger's env protocol is missing");
			std::process::exit(2);
		}
	};
	let root = PathBuf::from(root);

	if let Some(nodes) = std::env::var_os("ZIRKLE_NODES") {
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
	let host = zirkle::lua::Host::new(zirkle::runtime::Runtime::new(), &root, &root);
	let needs = zirkle::ledger::Ledger::scan(&root)
		.get(&node)
		.map(|e| e.needs.clone())
		.unwrap_or_default();
	let dep = std::env::var("ZIRKLE_DEP").unwrap_or_default();
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
	let serve_path = zirkle::socket::node_path(&root, &node);
	tokio::spawn({
		let host = host.clone();
		async move {
			if let Err(e) = zirkle::socket::serve(host, &serve_path).await {
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
	// dependency's seat: the dependent reads `ZIRKLE_DEP` and derives the
	// socket to call its needs over.
	let mut dependent: Option<tokio::process::Child> = None;
	if let Some(next) = chain.first() {
		let rest = serde_json::to_string(&chain[1..]).expect("node paths serialize");
		let mut reentry = tokio::process::Command::new(&zirkle);
		reentry
			.args(["enter", next, "--rest", &rest, "--dir", root.to_string_lossy().as_ref()])
			.env("ZIRKLE_ZIRKLE", &zirkle)
			.env("ZIRKLE_ROOT", &root)
			.env("ZIRKLE_DEP", &node)
			.stdin(Stdio::piped())
			.kill_on_drop(true);
		dependent = Some(reentry.spawn().expect("re-enter the resolver"));
	}

	// Stay up until the dependency that launched this node goes away, then
	// exit. The far end of the chain is different: nothing launched it that
	// owns it, so it has no pipe to watch and dies only when it is killed.
	// The held child goes with this node: the drop is what kills it.
	if std::env::var_os("ZIRKLE_BOTTOM").is_none() {
		let mut stdin = tokio::io::stdin();
		let mut buffer = [0u8; 64];
		while stdin.read(&mut buffer).await.unwrap_or(0) > 0 {}
	} else {
		futures::future::pending::<()>().await;
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
	let path = zirkle::socket::node_path(root, dep);
	let mut stream = None;
	for _ in 0..40 {
		match tokio::net::UnixStream::connect(&path).await {
			Ok(s) => {
				stream = Some(s);
				break;
			}
			Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
		}
	}
	let stream = stream
		.ok_or_else(|| format!("the dependency `{dep}` serves no socket at {}", path.display()))?;
	let (read, mut write) = stream.into_split();
	let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<serde_json::Value>>();
	let link = zirkle::cartridge::Link::new(tx, "the dependency is gone");
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
			zirkle::cartridge::Remote::over(link.clone(), key.to_owned()),
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
		.env("ZIRKLE_NODE", node)
		.env(
			"ZIRKLE_CHAIN",
			serde_json::to_string(rest).expect("node paths serialize"),
		)
		.env("ZIRKLE_ZIRKLE", exe())
		.env("ZIRKLE_ROOT", dir);
}

/// Start one node of the chain, detached. The invocation's job is to resolve
/// and hand off, and nothing sits above the tree owning it: the ask returns
/// while the tree keeps running, and the node's own death is what the rest
/// of the tree watches for.
fn launch_node(
	node: &zirkle::ledger::Installed,
	rest: &[&zirkle::ledger::Installed],
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
		.env("ZIRKLE_BOTTOM", "1")
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
	let (replies, mut pending) = tokio::sync::mpsc::channel::<String>(64);
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
			match host
				.call("mcp", json!({ "op": "message", "line": line }))
				.await
			{
				Ok(reply) if !reply.is_null() => {
					let _ = replies.send(format!("{reply}\n")).await;
				}
				Ok(_) => {}
				Err(error) => eprintln!("mcp: {error}"),
			}
		});
	}
	while serving.join_next().await.is_some() {}
	drop(replies);
	let _ = writer.await;
	Ok(Value::Null)
}

/// CLI service arguments use JSON.
fn json_arg(s: String) -> Value {
	serde_json::from_str(&s).unwrap_or_else(|error| {
		eprintln!("invalid JSON argument: {error}");
		std::process::exit(2);
	})
}

/// A request naming a turn gets it back on the reply, so a probe report can
/// cite the id of one specific call.
async fn ask(profile: &std::path::Path, request: Value, answer: &str) {
	let turn = request.get("turn").cloned();
	let mut client = connect(profile).await;
	client.send(request).await.expect("send");
	while let Some(mut m) = client.next().await {
		if m.get(answer).is_some() || m.get("error").is_some_and(|e| e.get("cartridge").is_some()) {
			if let (Some(turn), Some(o)) = (&turn, m.as_object_mut()) {
				o.insert("turn".into(), turn.clone());
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
			Some(p) if stack.contains(&p.entry.id) => println!("{pad}{key} <- {} (cycle)", p.entry.id),
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
		for note in [p.unread.as_deref(), p.error.as_deref()].into_iter().flatten() {
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
/// [`list`]: an unreadable document is listed and counted, never silently
/// absent and never printed as a cartridge that declared nothing, and a need
/// two entries of one scope offer is counted too, so a clash found at install
/// time is a non-zero exit rather than odd behaviour later.
fn ledger_lines(ledger: &zirkle::ledger::Ledger) -> usize {
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
				zirkle::ledger::Bound::One(p) => println!("  {key} <- {}", p.path),
				zirkle::ledger::Bound::None => println!("  {key} <- ?"),
				zirkle::ledger::Bound::Clashed(offered) => println!(
					"  {key} <- ambiguous ({})",
					offered.iter().map(|p| p.path.as_str()).collect::<Vec<_>>().join(", ")
				),
			}
		}
	}
	ledger
		.entries()
		.filter(|e| e.unread.is_some())
		.count()
		+ ledger
			.bindings()
			.iter()
			.filter(|(_, _, bound)| bound.is_clashed())
			.count()
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	if cli.yolo && !matches!(&cli.command, Command::Run { .. } | Command::Daemon) {
		eprintln!("--yolo requires run or daemon; it cannot change an existing daemon");
		std::process::exit(2);
	}
	let profile = loader::profile(cli.profile.as_deref().unwrap_or(match &cli.command {
		Command::Launch { .. } => "proxy",
		Command::Mcp => "mcp",
		_ => "default",
	}));
	let dir = cli.dir.unwrap_or_else(loader::builtin);
	match cli.command {
		Command::Run { key, args } => {
			// Foreground process cartridges receive terminal interrupts directly.
			let signals = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
			let host = Host::with_yolo(Runtime::new(), &dir, &profile, cli.yolo);
			// The live host also answers on its socket, so `zirkle call`, `status`
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
					zirkle::turn::diagnostic("zirkle", error, Value::Null);
					std::process::exit(1);
				}
			}
		}
		Command::Launch { agent, model, args } => {
			// The proxy listener needs a key; the launched agent gets the same one.
			if std::env::var("ZIRKLE_PROXY_KEY").map_or(true, |k| k.is_empty()) {
				std::env::set_var("ZIRKLE_PROXY_KEY", random_hex(32));
			}
			let signals = tokio::spawn(async { while tokio::signal::ctrl_c().await.is_ok() {} });
			let host = Host::new(Runtime::new(), &dir, &profile);
			let request = json!({ "op": "launch", "agent": agent, "model": model, "args": args });
			let result = host.run_then("proxy", request, spawn).await;
			signals.abort();
			match result {
				Ok(status) => std::process::exit(status.as_i64().unwrap_or(1) as i32),
				Err(error) => {
					zirkle::turn::diagnostic("zirkle", error, Value::Null);
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
				zirkle::turn::diagnostic("init.lua", e, Value::Null);
			}
			if let Err(e) = host.watch() {
				zirkle::turn::diagnostic("watch", e, Value::Null);
			}
			let path = socket::path(&profile);
			zirkle::turn::diagnostic(
				"zirkle",
				"serving",
				json!({ "dir": dir.display().to_string(), "profile": profile.display().to_string(), "socket": path.display().to_string() }),
			);
			if let Err(e) = socket::serve(host, &path).await {
				zirkle::turn::diagnostic("zirkle", e, Value::Null);
				std::process::exit(1);
			}
		}
		Command::Send { name, data } => {
			connect(&profile)
				.await
				.send(json!({ "emit": name, "data": json_arg(data) }))
				.await
				.expect("send");
		}
		Command::Tail => {
			let mut client = connect(&profile).await;
			while let Some(m) = client.next().await {
				println!("{m}");
			}
		}
		Command::Call { key, args, turn } => {
			let turn = turn.unwrap_or_else(|| zirkle::turn::mint().to_string());
			ask(
				&profile,
				json!({ "call": key, "args": json_arg(args), "id": 1, "turn": turn }),
				"reply",
			)
			.await;
		}
		Command::Reload => ask(&profile, json!({ "reload": true }), "reloaded").await,
		Command::Status => ask(&profile, json!({ "status": true }), "status").await,
		Command::Debug { state } => ask(&profile, json!({ "debug": state }), "debug").await,
		Command::Socket => println!("{}", socket::path(&profile).display()),
		Command::Verify => {
			let host = Host::new(Runtime::new(), &dir, &profile);
			match host.verify().await {
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
			let ledger = zirkle::ledger::Ledger::scan(&dir);
			if ledger_lines(&ledger) > 0 {
				std::process::exit(1);
			}
		}
		Command::Up { key } => {
			let ledger = zirkle::ledger::Ledger::scan(&dir);
			// One ask, one resolution: the walk runs to the far end here, and
			// every link after it is a re-entry that re-reads the ledger. A
			// refusal is a refusal of the launch: nothing spawns, and the exit
			// says so.
			match zirkle::resolver::chain(&ledger, "", &key) {
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
					let (bottom, rest) = chain.split_first().expect("a resolved chain is never empty");
					launch_node(bottom, rest, &dir);
					// The tree answers at the top: the socket the named tool's
					// node serves is where its keys are called, derived by
					// anyone from the root and the node's ledger path.
					let top = chain.last().expect("a resolved chain is never empty");
					println!(
						"{}",
						json!({ "up": key, "nodes": chain.iter().map(|n| n.path.clone()).collect::<Vec<_>>(), "socket": zirkle::socket::node_path(&dir, &top.path) })
					);
				}
			}
		}
		Command::Enter { node, rest } => {
			let ledger = zirkle::ledger::Ledger::scan(&dir);
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
				.env("ZIRKLE_NODE", &node)
				.env("ZIRKLE_CHAIN", rest)
				.env("ZIRKLE_ZIRKLE", exe())
				.env("ZIRKLE_ROOT", &dir)
				// The far end was the only node nothing launched; a node
				// re-entered into has a dependency above it and loses the flag.
				.env_remove("ZIRKLE_BOTTOM")
				// The node's stdin is the dependency's pipe, inherited: when
				// the dependency that launched it goes away, the pipe closes
				// and EOF is the signal to take the rest of the tree along.
				.current_dir(&entry.dir);
			let error = command.exec();
			eprintln!("{node}: {error}");
			std::process::exit(1);
		}
		Command::Node => hosted_node().await,
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
