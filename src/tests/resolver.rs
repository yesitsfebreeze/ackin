//! The resolver: the launch walk over the ledger. The chain is the launch
//! plan, bottom-up; a cycle is refused; a clash is refused naming every
//! offer; a key nothing offers refuses the launch. The end-to-end tests run
//! the binary the way a person does and assert the tree the launch leaves
//! behind — once through the program fixture a document's `binary` names,
//! and once through the hosted route, where a Lua-entry cartridge with no
//! `binary` is the program the host itself runs.

use super::*;
use crate::ledger::Ledger;
use crate::resolver::{chain, Refusal};
use serde_json::json;

fn cartridge(dir: &Path, folder: &str, manifest: Value) {
	std::fs::create_dir_all(dir.join(folder)).unwrap();
	write(
		dir,
		&format!("{folder}/cartridge.json"),
		&manifest.to_string(),
	);
	write(dir, &format!("{folder}/init.lua"), "return {}");
}

/// The chain reads bottom-up: the far end first, the provider of the named
/// tool last, and every node's own needs resolved out of the asker's own
/// subtree — the identical key one subtree over is not a candidate.
#[test]
fn a_chain_walks_to_its_far_end_before_the_tool_it_names() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"tool",
		json!({"name": "tool", "entry": "init.lua", "provide": ["tool.run"], "needs": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"store",
		json!({"name": "store", "entry": "init.lua", "provide": ["store.get"], "needs": ["db.key"]}),
	);
	cartridge(
		dir.path(),
		"db",
		json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
	);
	let ledger = Ledger::scan(dir.path());
	let chain = chain(&ledger, "", "tool.run").unwrap();
	assert_eq!(
		chain.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
		vec!["db", "store", "tool"],
		"the far end first, and the stranger never a candidate"
	);
}

/// A key one subtree over is invisible however identical its name: the walk
/// binds from the asking node's own subtree first and steps outward, so a
/// nested cartridge is launched, not its flat-registry lookalike.
#[test]
fn a_nested_provider_answers_before_any_outer_one() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"outer",
		json!({"name": "outer", "entry": "init.lua", "provide": ["outer.run"], "needs": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"outer/inner",
		json!({"name": "inner", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"other",
		json!({"name": "other", "entry": "init.lua", "provide": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	let chain = chain(&ledger, "", "outer.run").unwrap();
	assert_eq!(
		chain.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
		vec!["outer/inner", "outer"]
	);
}

/// A diamond is not a cycle: two nodes may ask for the same provider, and
/// the chain carries it once.
#[test]
fn a_diamond_launches_its_shared_provider_once() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"top",
		json!({"name": "top", "entry": "init.lua", "provide": ["top.run"], "needs": ["left.key", "right.key"]}),
	);
	cartridge(
		dir.path(),
		"left",
		json!({"name": "left", "entry": "init.lua", "provide": ["left.key"], "needs": ["shared.key"]}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua", "provide": ["right.key"], "needs": ["shared.key"]}),
	);
	cartridge(
		dir.path(),
		"shared",
		json!({"name": "shared", "entry": "init.lua", "provide": ["shared.key"]}),
	);
	let ledger = Ledger::scan(dir.path());
	let chain = chain(&ledger, "", "top.run").unwrap();
	assert_eq!(
		chain.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
		vec!["shared", "left", "right", "top"]
	);
}

/// A chain that closes on a node it is already walking through is refused,
/// the duty `deps` already carried: the launch path inherits it.
#[test]
fn a_cycle_is_refused_not_walked() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"a",
		json!({"name": "a", "entry": "init.lua", "provide": ["a.key"], "needs": ["b.key"]}),
	);
	cartridge(
		dir.path(),
		"b",
		json!({"name": "b", "entry": "init.lua", "provide": ["b.key"], "needs": ["a.key"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match chain(&ledger, "", "a.key") {
		Err(refusal) => {
			assert_eq!(
				refusal,
				Refusal::Cycle {
					key: "a.key".into(),
					at: "a".into()
				}
			)
		}
		Ok(launched) => panic!("a cycle resolved: {:?}", launched.iter().map(|e| e.path.clone()).collect::<Vec<_>>()),
	}
}

/// A key one scope offers twice is a clash, and the refusal names every
/// offer — the launch inherits the listing's duty to stop rather than pick.
#[test]
fn a_clash_is_refused_naming_every_offer() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"left",
		json!({"name": "left", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"right",
		json!({"name": "right", "entry": "init.lua", "provide": ["store.get"]}),
	);
	cartridge(
		dir.path(),
		"tool",
		json!({"name": "tool", "entry": "init.lua", "provide": ["tool.run"], "needs": ["store.get"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match chain(&ledger, "", "tool.run") {
		Err(refusal) => {
			assert_eq!(
				refusal,
				Refusal::Ambiguous {
					key: "store.get".into(),
					offered: vec!["left".into(), "right".into()]
				}
			)
		}
		Ok(launched) => panic!("a clash resolved: {:?}", launched.iter().map(|e| e.path.clone()).collect::<Vec<_>>()),
	}
}

/// A key nothing offers is legal to list and not legal to launch: the ask
/// fails naming what it asked for and where, which is the absence of a
/// launch an uninstall reads as.
#[test]
fn an_unbound_key_refuses_the_launch() {
	let dir = tempfile::tempdir().unwrap();
	cartridge(
		dir.path(),
		"tool",
		json!({"name": "tool", "entry": "init.lua", "provide": ["tool.run"], "needs": ["absent.key"]}),
	);
	let ledger = Ledger::scan(dir.path());
	match chain(&ledger, "", "tool.run") {
		Err(refusal) => {
			assert_eq!(
				refusal,
				Refusal::Unbound {
					from: "tool".into(),
					key: "absent.key".into()
				}
			)
		}
		Ok(launched) => panic!("an unbound ask resolved: {:?}", launched.iter().map(|e| e.path.clone()).collect::<Vec<_>>()),
	}
}

// --- the tree ---

/// One chain node per manifest, all standing in as the program the document
/// names. `bin/` inside the cartridge folder is the first place the resolver
/// looks, so the fixture is linked under each node's own name.
fn chain_of(dir: &Path, manifests: &[(&str, Value)]) {
	for (folder, manifest) in manifests {
		let mut document = manifest.clone();
		document["binary"] = json!(format!("{folder}_node"));
		cartridge(dir, folder, document);
		let bin = dir.join(folder).join("bin");
		std::fs::create_dir_all(&bin).unwrap();
		std::os::unix::fs::symlink(fixture_path(), bin.join(format!("{folder}_node"))).unwrap();
	}
}

/// The hosted node's in-host step, without the process boundary around it:
/// the component loads the way `enter`'s node mode loads it, its needs are
/// the chain's business and not injections — a hosted node whose document
/// declares needs still runs its apply, because the dependency that
/// satisfies them is the process that launched it — and the apply's
/// `ctx.provide` registers the key the document declared, the way an
/// in-host fiber provides.
#[tokio::test(flavor = "multi_thread")]
async fn a_hosted_node_runs_its_lua_component_and_provides_its_keys() {
	let dir = tempfile::tempdir().unwrap();
	hosted(
		dir.path(),
		"db",
		json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
	);
	hosted(
		dir.path(),
		"store",
		json!({"name": "store", "entry": "init.lua", "provide": ["store.get"], "needs": ["db.key"]}),
	);
	let host = crate::lua::Host::new(Runtime::new(), dir.path(), dir.path());
	for (folder, key) in [("db", "db.key"), ("store", "store.get")] {
		let mut component = host
			.component(&dir.path().join(folder), Value::Null)
			.unwrap_or_else(|e| panic!("{folder}: {e}"));
		assert_eq!(component.provide, vec![key]);
		assert_eq!(component.inject.len(), if folder == "db" { 0 } else { 1 });
		// What the node mode does: the chain is the resolution of the needs,
		// so they are not injections waiting on providers in this process.
		component.inject.clear();
		let fiber = host.runtime().ctx().cartridge(component);
		assert!(
			wait_for(|| fiber.state() == Some(crate::runtime::State::Active), 5),
			"{folder} came up {:?} ({:?})",
			fiber.state(),
			fiber.error()
		);
		assert!(
			host.runtime().ctx().peek(key).is_some(),
			"{folder} provides `{key}`"
		);
	}
}

/// A chain that asks for a key the top node provides: db <- store <- tool.
fn store_chain(dir: &Path) {
	chain_of(
		dir,
		&[
			(
				"tool",
				json!({"name": "tool", "entry": "init.lua", "provide": ["tool.run"], "needs": ["store.get"]}),
			),
			(
				"store",
				json!({"name": "store", "entry": "init.lua", "provide": ["store.get"], "needs": ["db.key"]}),
			),
			(
				"db",
				json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
			),
		],
	);
}

fn fixture_path() -> std::path::PathBuf {
	static FIXTURE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	FIXTURE
		.get_or_init(|| built(&["-p", "zirkle", "--example", "chain_fixture"]))
		.clone()
}

pub fn zirkle_path() -> std::path::PathBuf {
	static ZIRKLE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
	ZIRKLE.get_or_init(|| built(&["-p", "zirkle"])).clone()
}

/// The pids the probe's nodes wrote, keyed by the node path they stood for.
pub fn pids(dir: &Path, nodes: &[&str]) -> Vec<u32> {
	try_pids(dir, nodes).expect("every node marked its pid")
}

pub fn try_pids(dir: &Path, nodes: &[&str]) -> Option<Vec<u32>> {
	nodes
		.iter()
		.map(|node| {
			let marker = dir.join(".nodes").join(node.replace('/', "_"));
			std::fs::read_to_string(&marker)
				.ok()?
				.trim()
				.parse()
				.ok()
		})
		.collect()
}

/// A node's parent in the process tree is the dependency that launched it.
pub fn ppid(pid: u32) -> u32 {
	let out = std::process::Command::new("ps")
		.args(["-o", "ppid=", "-p", &pid.to_string()])
		.output()
		.unwrap();
	String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

pub fn alive(pid: u32) -> bool {
	std::process::Command::new("ps")
		.args(["-p", &pid.to_string()])
		.output()
		.map(|o| String::from_utf8_lossy(&o.stdout).lines().count() > 1)
		.unwrap_or(false)
}

pub fn wait_for(condition: impl Fn() -> bool, seconds: u64) -> bool {
	for _ in 0..seconds * 20 {
		if condition() {
			return true;
		}
		std::thread::sleep(std::time::Duration::from_millis(50));
	}
	condition()
}

/// A chain of ordinary Lua-entry cartridges: no `binary` anywhere, so every
/// node is hosted — the binary itself is the program, and each entry's apply
/// provides the key its document declares the way an in-host fiber does.
fn hosted(dir: &Path, folder: &str, manifest: Value) {
	let key = manifest["provide"]
		.as_array()
		.and_then(|keys| keys.first())
		.and_then(Value::as_str)
		.unwrap_or_default()
		.to_owned();
	cartridge(dir, folder, manifest);
	write(
		dir,
		&format!("{folder}/init.lua"),
		&format!(
			"return {{ apply = function(ctx) ctx:provide(\"{key}\", function() return {{}} end) end }}"
		),
	);
}

/// A document whose `binary` names a program that does not exist refuses the
/// ask naming the node and the missing program, before anything of the chain
/// spawns.
#[tokio::test(flavor = "multi_thread")]
async fn a_chain_node_whose_binary_does_not_exist_refuses_the_ask() {
	let dir = tempfile::tempdir().unwrap();
	hosted(
		dir.path(),
		"db",
		json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
	);
	cartridge(
		dir.path(),
		"store",
		json!({"name": "store", "entry": "init.lua", "binary": "gone_node", "provide": ["store.get"], "needs": ["db.key"]}),
	);
	let nodes_dir = dir.path().join(".nodes");
	std::fs::create_dir_all(&nodes_dir).unwrap();
	let ask = std::process::Command::new(zirkle_path())
		.args(["up", "store.get", "--dir", &dir.path().to_string_lossy()])
		.env("ZIRKLE_NODES", &nodes_dir)
		.output()
		.unwrap();
	assert!(!ask.status.success());
	let stderr = String::from_utf8_lossy(&ask.stderr);
	assert!(
		stderr.contains("store") && stderr.contains("gone_node"),
		"the refusal names the node and the missing program: {stderr}"
	);
	// The far end is still there to be asked for: nothing of the chain came up.
	assert!(
		std::fs::read_dir(&nodes_dir).unwrap().next().is_none(),
		"nothing of the chain spawned"
	);
}

/// A node uninstalled between the ask and the re-entry launches nothing: the
/// re-entry re-reads the ledger, and a node it no longer holds is no
/// launch, however recently the chain resolved it — the refusal names the
/// node.
#[tokio::test(flavor = "multi_thread")]
async fn a_reentry_into_an_uninstalled_node_launches_nothing() {
	let dir = tempfile::tempdir().unwrap();
	let root = dir.path().to_path_buf();
	hosted(
		&root,
		"db",
		json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
	);
	// Uninstalled after the ask resolved it, before the re-entry reads.
	std::fs::remove_dir_all(root.join("db")).unwrap();
	let enter = std::process::Command::new(zirkle_path())
		.args(["enter", "db", "--rest", "[]", "--dir", &root.to_string_lossy()])
		.env("ZIRKLE_ZIRKLE", zirkle_path())
		.env("ZIRKLE_ROOT", &root)
		.output()
		.unwrap();
	assert!(!enter.status.success());
	let stderr = String::from_utf8_lossy(&enter.stderr);
	assert!(
		stderr.contains("db") && stderr.contains("no longer installed"),
		"the re-entry names the node it refuses to launch: {stderr}"
	);
}

/// A chain of ordinary Lua-entry cartridges — no `binary` anywhere — comes up
/// from the bottom under one ask, the host process is each chain node, and
/// the cascade holds through a hosted node: the far end is killed, its
/// dependents go with it, and an uninstalled far end launches nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency() {
	let dir = tempfile::tempdir().unwrap();
	let root = dir.path().to_path_buf();
	hosted(
		&root,
		"db",
		json!({"name": "db", "entry": "init.lua", "provide": ["db.key"]}),
	);
	hosted(
		&root,
		"store",
		json!({"name": "store", "entry": "init.lua", "provide": ["store.get"], "needs": ["db.key"]}),
	);
	hosted(
		&root,
		"tool",
		json!({"name": "tool", "entry": "init.lua", "provide": ["tool.run"], "needs": ["store.get"]}),
	);
	let nodes_dir = root.join(".nodes");
	std::fs::create_dir_all(&nodes_dir).unwrap();

	let ask = tokio::process::Command::new(zirkle_path())
		.args(["up", "tool.run", "--dir", &root.to_string_lossy()])
		.env("ZIRKLE_NODES", &nodes_dir)
		.spawn()
		.unwrap()
		.wait()
		.await
		.unwrap();
	assert!(ask.success(), "the ask resolves and hands off: {ask:?}");

	// One host process per node, up from the bottom, and each node's parent
	// in the process tree is the dependency that launched it — the same tree
	// the program fixture leaves behind.
	assert!(
		wait_for(
			|| try_pids(&root, &["db", "store", "tool"]).is_some_and(|p| p.len() == 3),
			10
		),
		"the hosted chain came up: {:?}",
		try_pids(&root, &["db", "store", "tool"])
	);
	let (db, store, tool) = {
		let p = pids(&root, &["db", "store", "tool"]);
		(p[0], p[1], p[2])
	};
	assert_eq!(ppid(store), db, "the dependency launched its dependent");
	assert_eq!(ppid(tool), store, "and so on up the chain");

	// Teardown is the dependency's exit, and it is ordered for free: the
	// cascade runs through a hosted node, not only through the fixture.
	std::process::Command::new("kill")
		.args(["-9", &db.to_string()])
		.status()
		.unwrap();
	assert!(
		wait_for(|| !alive(store) && !alive(tool), 10),
		"everything that needed the hosted node went with it"
	);

	// Uninstalling is the absence of a launch, on the hosted route as on the
	// fixture's: with the far end gone, the fresh ledger binds nothing.
	std::fs::remove_dir_all(root.join("db")).unwrap();
	let ask = std::process::Command::new(zirkle_path())
		.args(["up", "tool.run", "--dir", &root.to_string_lossy()])
		.output()
		.unwrap();
	assert!(!ask.status.success());
	assert!(
		String::from_utf8_lossy(&ask.stderr).contains("binds to nothing"),
		"{}",
		String::from_utf8_lossy(&ask.stderr)
	);
}

/// Starting a tool brings its whole chain up from the bottom, and because a
/// dependency launches its dependent, the process tree and the dependency
/// tree are the same tree. A node killed takes everything that needed it
/// with it, and a second ask for a tool the ledger no longer holds launches
/// nothing at all.
#[tokio::test(flavor = "multi_thread")]
async fn a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency() {
	let dir = tempfile::tempdir().unwrap();
	let root = dir.path().to_path_buf();
	store_chain(&root);
	let nodes_dir = root.join(".nodes");
	std::fs::create_dir_all(&nodes_dir).unwrap();

	let ask = tokio::process::Command::new(zirkle_path())
		.args(["up", "tool.run", "--dir", &root.to_string_lossy()])
		.env("ZIRKLE_NODES", &nodes_dir)
		.output()
		.await
		.unwrap();
	// The ask's stdout is captured here, so this returning at all is the
	// hand-off's proof: a detached node holding the asker's pipe would hold
	// this test open forever.
	assert!(ask.status.success(), "the ask resolves and hands off: {ask:?}");

	// One process per node, up from the bottom, and each node's parent in
	// the process tree is the dependency that launched it.
	assert!(
		wait_for(
			|| try_pids(&root, &["db", "store", "tool"]).is_some_and(|p| p.len() == 3),
			10
		),
		"the chain came up: {:?} {:?}",
		try_pids(&root, &["db", "store", "tool"]),
		std::fs::read_dir(&nodes_dir)
			.map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>())
			.unwrap_or_default()
	);
	let (db, store, tool) = {
		let p = pids(&root, &["db", "store", "tool"]);
		(p[0], p[1], p[2])
	};
	assert_eq!(ppid(store), db, "the dependency launched its dependent");
	assert_eq!(ppid(tool), store, "and so on up the chain");

	// Teardown is the dependency's exit, and it is ordered for free: the
	// dependents go with the node they needed.
	std::process::Command::new("kill")
		.args(["-9", &db.to_string()])
		.status()
		.unwrap();
	assert!(
		wait_for(|| !alive(store) && !alive(tool), 10),
		"everything that needed the node went with it"
	);

	// Uninstalling is the absence of a launch: with the far end gone, the
	// fresh ledger binds nothing, and the ask refuses rather than spawning.
	std::fs::remove_dir_all(root.join("db")).unwrap();
	let ask = std::process::Command::new(zirkle_path())
		.args(["up", "tool.run", "--dir", &root.to_string_lossy()])
		.output()
		.unwrap();
	assert!(!ask.status.success());
	assert!(
		String::from_utf8_lossy(&ask.stderr).contains("binds to nothing"),
		"{}",
		String::from_utf8_lossy(&ask.stderr)
	);
}

/// A cycle in the chain is still detected and still refused on the launch
/// path, not only in the listing.
#[tokio::test(flavor = "multi_thread")]
async fn the_launch_refuses_a_cycle_the_ledger_holds() {
	let dir = tempfile::tempdir().unwrap();
	chain_of(
		dir.path(),
		&[
			(
				"a",
				json!({"name": "a", "entry": "init.lua", "provide": ["a.key"], "needs": ["b.key"]}),
			),
			(
				"b",
				json!({"name": "b", "entry": "init.lua", "provide": ["b.key"], "needs": ["a.key"]}),
			),
		],
	);
	let ask = std::process::Command::new(zirkle_path())
		.args(["up", "a.key", "--dir", &dir.path().to_string_lossy()])
		.output()
		.unwrap();
	assert!(!ask.status.success());
	assert!(
		String::from_utf8_lossy(&ask.stderr).contains("cycle"),
		"{}",
		String::from_utf8_lossy(&ask.stderr)
	);
}