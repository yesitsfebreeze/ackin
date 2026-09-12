use std::path::PathBuf;

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
	/// Load the profile and run every contract its cartridges declare
	Verify,
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
