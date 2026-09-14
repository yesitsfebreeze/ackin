//! The chain: `up` resolves a tool's chain and launches its far end, `enter`
//! is the re-entry each node execs into for the next link, and the hosted
//! node mode runs a Lua-entry cartridge in-host as one link of the tree.

use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::sync::Arc;

use cartridge::lua::Host;
use cartridge::{Error, Result};
use serde_json::json;

use super::{exe, fail, stopped, Project, FAILED, USAGE};

/// What a chain node runs. A document that declares a `binary` names its own
/// program, resolved the way every process component resolves its command —
/// the cartridge's own `bin/`, then beside the running cartridge, then `PATH`.
/// A document that declares none is **hosted**: the binary itself is the
/// program, and the node mode runs the cartridge's Lua component in-host.
enum Route {
	Program(PathBuf),
	Hosted,
}

impl Route {
	/// The command this route launches, before its environment and stdio.
	fn command(self) -> std::process::Command {
		match self {
			Route::Program(program) => std::process::Command::new(program),
			// The hosted node's program is this binary itself, in the node mode
			// the re-entry execs into.
			Route::Hosted => {
				let mut command = std::process::Command::new(exe());
				command.arg("node");
				command
			}
		}
	}
}

/// What a chain node launches, resolved the way the ask and every re-entry
/// resolve it — once, so `hello`, the ask and the hand-off name the same
/// program. A document that would not read, and a `binary` that does not
/// exist, refuse naming the node; the ask checks every node before the first
/// one spawns.
fn route(node: &cartridge::ledger::Installed) -> Result<Route> {
	let manifest = node.dir.join(cartridge::loader::MANIFEST);
	let at = |e: Error| Error::Profile(format!("`{}`: {e}", node.path));
	let document = cartridge::loader::Cartridge::document(&manifest).map_err(at)?;
	match document.binary {
		Some(binary) => cartridge::cartridge::executable(&binary, &node.dir)
			.map(Route::Program)
			.map_err(|e| at(Error::Io(e))),
		// The cartridge has no program of its own, so the host is it: the
		// entry is resolved now, not evaluated — a missing Lua file is a
		// refusal of the ask, an entry that fails to apply is the node's.
		None => cartridge::loader::Cartridge::read(&manifest)
			.map(|_| Route::Hosted)
			.map_err(at),
	}
}

/// One ask, one resolution: the walk runs to the far end here, and every
/// link after it is a re-entry that re-reads the ledger. A refusal is a
/// refusal of the launch: nothing spawns, and the exit says so.
pub(crate) fn up(project: &Project, key: &str) -> Result<ExitCode> {
	let ledger = cartridge::ledger::Ledger::scan(&project.dir);
	let chain = cartridge::resolver::chain(&ledger, "", key)?;
	// Every node of the chain is checked before the first one spawns: a
	// refusal belongs to the ask, not to a link that already came up.
	for node in &chain {
		route(node)?;
	}
	let (bottom, rest) = chain
		.split_first()
		.expect("a resolved chain is never empty");
	launch_node(bottom, rest, &project.dir)?;
	// The tree answers at the top: the socket the named tool's node serves
	// is where its keys are called, derived by anyone from the root and the
	// node's ledger path.
	let top = chain.last().expect("a resolved chain is never empty");
	println!(
		"{}",
		json!({ "up": key, "nodes": chain.iter().map(|n| n.path.clone()).collect::<Vec<_>>(), "socket": cartridge::socket::node_path(&project.dir, &top.path) })
	);
	Ok(ExitCode::SUCCESS)
}

/// The environment every node of the tree carries: its identity in the
/// ledger, what is still to come after it, and how to re-enter the binary.
/// The chain rides in the environment, not in a coordinator: the ask
/// resolved it, and each node hands its remainder to its dependent.
fn node_env(command: &mut std::process::Command, node: &str, rest: &str, dir: &Path) {
	command
		.env("CARTRIDGE_NODE", node)
		.env("CARTRIDGE_CHAIN", rest)
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
) -> Result<()> {
	let paths: Vec<String> = rest.iter().map(|e| e.path.clone()).collect();
	let rest = serde_json::to_string(&paths).expect("node paths serialize");
	let mut command = tokio::process::Command::from(route(node)?.command());
	command.current_dir(&node.dir);
	node_env(command.as_std_mut(), &node.path, &rest, dir);
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
	let child = command.spawn().map_err(|e| Error::process(&node.path, e))?;
	println!("{}", child.id().unwrap_or(0));
	Ok(())
}

/// The step re-reads the ledger: a node that is no longer installed has no
/// launch, however recently the chain held it. A document that declares a
/// binary execs into it; one that does not is hosted, and the host's own
/// node mode is the program.
pub(crate) fn enter(project: &Project, node: &str, rest: &str) -> ExitCode {
	use std::os::unix::process::CommandExt;
	let ledger = cartridge::ledger::Ledger::scan(&project.dir);
	let Some(entry) = ledger.get(node) else {
		return fail(
			FAILED,
			format!("`{node}` is no longer installed: nothing launches"),
		);
	};
	let mut command = match route(entry) {
		Ok(route) => route.command(),
		Err(e) => return fail(FAILED, e),
	};
	node_env(&mut command, node, rest, &project.dir);
	command
		// The far end was the only node nothing launched; a node
		// re-entered into has a dependency above it and loses the flag.
		.env_remove("CARTRIDGE_BOTTOM")
		// The node's stdin is the dependency's pipe, inherited: when
		// the dependency that launched it goes away, the pipe closes
		// and EOF is the signal to take the rest of the tree along.
		.current_dir(&entry.dir);
	let error = command.exec();
	fail(FAILED, format!("{node}: {error}"))
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
pub(crate) async fn hosted(_project: &Project) -> Result<ExitCode> {
	use tokio::io::AsyncReadExt;

	let node = std::env::var("CARTRIDGE_NODE").unwrap_or_default();
	let chain: Vec<String> =
		serde_json::from_str(&std::env::var("CARTRIDGE_CHAIN").unwrap_or_default())
			.unwrap_or_default();
	let (cartridge, root) = match (
		std::env::var("CARTRIDGE_CARTRIDGE"),
		std::env::var("CARTRIDGE_ROOT"),
	) {
		(Ok(cartridge), Ok(root)) => (cartridge, PathBuf::from(root)),
		_ => {
			return Ok(fail(
				USAGE,
				"node mode is entered, not asked: the ledger's env protocol is missing",
			))
		}
	};

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
	let host = Host::new(cartridge::runtime::Runtime::new(), &root, &root);
	let needs = cartridge::ledger::Ledger::scan(&root)
		.get(&node)
		.map(|e| e.needs.clone())
		.unwrap_or_default();
	let dep = std::env::var("CARTRIDGE_DEP").unwrap_or_default();
	let dep = dep.trim();
	let at = |e: Error| Error::Profile(format!("`{node}`: {e}"));
	if !dep.is_empty() {
		bind_dependency(&host, &root, dep, &needs)
			.await
			.map_err(at)?;
	}
	let mut component = host
		.component(&root.join(&node), serde_json::Value::Null)
		.map_err(at)?;
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
				tracing::error!(target: "cartridge", "node socket: {e}");
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
		dependent =
			Some(reentry.spawn().map_err(|e| {
				Error::process(&cartridge, format!("re-entering the resolver: {e}"))
			})?);
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
	Ok(ExitCode::SUCCESS)
}

/// Bind the node's needs to its dependency: a client on the dependency's
/// socket, every need of the document a remote over it. The dependency served
/// before it re-entered, but the door is only ever a spawn away — a short
/// retry, then a refusal that names the link. The link dies with the
/// socket: a dependency that goes away fails every call still in flight and
/// the stdin EOF takes the node along.
async fn bind_dependency(host: &Arc<Host>, root: &Path, dep: &str, needs: &[String]) -> Result<()> {
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
	let stream = stream.ok_or_else(|| Error::Unavailable {
		key: dep.to_owned(),
		why: format!(
			"the dependency `{dep}` serves no socket at {}",
			path.display()
		),
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
