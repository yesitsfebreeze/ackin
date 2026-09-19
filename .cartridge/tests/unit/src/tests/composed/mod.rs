//! The parent's four acceptance claims, composed. Every count here is an exact
//! equality against a non-zero number measured in the lab's own run directory;
//! no bound, and nothing is counted in the project's `.cartridge`.
mod fixture;

use fixture::Lab;

/// Claim 1: with a daemon running, a `run` answers off the daemon's nodes and
/// starts none of its own.
#[test]
fn a_run_against_a_live_daemon_starts_no_node() {
	let mut lab = Lab::new("run");
	let root = lab.root.clone();
	lab.start(&root);
	assert_eq!(lab.settled(&root), 3);
	let daemon = lab.daemon(&root);
	assert_ne!(daemon, 0);
	assert_eq!(lab.compositions(&root), 1);
	assert_eq!(lab.nodes(&root), 3);
	let before = lab.call(&root, "store", r#"{"op":"query","text":"x"}"#)["node"].clone();

	let out = lab.cli(&root, &["run", "store", r#"{"op":"query","text":"x"}"#]);
	assert!(
		out.status.success(),
		"{}",
		String::from_utf8_lossy(&out.stderr)
	);
	let reply: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

	assert_eq!(lab.compositions(&root), 1);
	assert_eq!(lab.nodes(&root), 3);
	assert_eq!(lab.daemon(&root), daemon);
	assert_eq!(
		reply["node"], before,
		"the run was served by a node of its own"
	);
}

/// Claim 2: two launches and one mcp leave one daemon and one node per
/// composed cartridge.
#[test]
fn two_launches_and_one_mcp_leave_one_daemon_and_one_node_per_cartridge() {
	let mut lab = Lab::new("instances");
	let root = lab.root.clone();
	lab.start(&root);
	assert_eq!(lab.settled(&root), 3);
	let daemon = lab.daemon(&root);

	let a = lab.spawn(&root, &["launch", "alpha"]);
	let b = lab.spawn(&root, &["launch", "beta"]);
	let m = lab.spawn(&root, &["mcp"]);
	assert_ne!(a, daemon);
	assert_ne!(b, daemon);
	assert_ne!(m, daemon);
	// Wait for the two launches by the identity each one carries, never by the
	// count this test is about to assert on. A wait that stops at the first
	// number clearing a threshold is satisfied by a node that miscounts, and
	// the assertion then reads the very value that ended the wait.
	let mut agents: Vec<String> = Vec::new();
	for _ in 0..300 {
		agents = lab.call(&root, "proxy", r#"{"op":"count"}"#)["agents"]
			.as_array()
			.map(|all| {
				all.iter()
					.filter_map(|a| a.as_str().map(str::to_owned))
					.collect()
			})
			.unwrap_or_default();
		if agents.iter().any(|a| a == "alpha") && agents.iter().any(|a| a == "beta") {
			break;
		}
		std::thread::sleep(std::time::Duration::from_millis(100));
	}
	assert!(
		agents.iter().any(|a| a == "alpha") && agents.iter().any(|a| a == "beta"),
		"both launches must have reached the one proxy node, which saw {agents:?}"
	);
	let launches = lab.call(&root, "proxy", r#"{"op":"count"}"#)["count"]
		.as_u64()
		.unwrap_or(0);
	assert_eq!(
		launches, 2,
		"the one proxy node must count exactly the two launches it saw"
	);
	assert!(
		alive(a) && alive(b) && alive(m),
		"all three instances must be up at once"
	);

	assert_eq!(lab.compositions(&root), 1);
	assert_eq!(lab.nodes(&root), 3);
	assert_eq!(lab.daemon(&root), daemon);

	// Negative control, measured by the same two helpers: a project that really
	// does hold a second composition reports 2. A helper pointed at the wrong
	// directory reports 0 here and fails, which is what the sibling's
	// bounded assertion could never do.
	let other = lab.project();
	lab.start(&other);
	assert_eq!(lab.settled(&other), 3);
	let both = lab.compositions(&root) + lab.compositions(&other);
	assert_eq!(both, 2);
	assert_ne!(lab.daemon(&other), daemon);
}

/// Claim 3: one instance's write is another instance's read, off one node.
#[test]
fn one_instance_writes_what_another_reads_off_one_node() {
	let mut lab = Lab::new("shared");
	let root = lab.root.clone();
	lab.start(&root);
	assert_eq!(lab.settled(&root), 3);
	assert_ne!(lab.daemon(&root), 0);

	// Two separate processes: one ingests, the other queries.
	let wrote = lab.call(&root, "store", r#"{"op":"ingest","text":"cedar"}"#);
	let read = lab.call(&root, "store", r#"{"op":"query","text":"cedar"}"#);
	assert_eq!(
		read["hit"], true,
		"the second instance did not see the first's write"
	);
	assert_eq!(
		read["node"], wrote["node"],
		"two nodes served the two instances"
	);

	let miss = lab.call(&root, "store", r#"{"op":"query","text":"larch"}"#);
	assert_eq!(
		miss["hit"], false,
		"the store answers hit for anything it was never given"
	);

	assert_eq!(lab.compositions(&root), 1);
	assert_eq!(lab.nodes(&root), 3);
}

/// Claim 4: with no daemon, two instances started at once end up on one.
#[test]
fn two_instances_started_cold_and_concurrently_share_one_daemon() {
	let mut lab = Lab::new("cold");
	let root = lab.root.clone();
	assert_eq!(lab.compositions(&root), 0, "the lab must start cold");

	let first = lab.spawn(&root, &["run", "store", r#"{"op":"query","text":"y"}"#]);
	let second = lab.spawn(&root, &["run", "store", r#"{"op":"query","text":"y"}"#]);
	assert_ne!(first, second);
	assert_eq!(lab.settled(&root), 3);

	let a = lab.call(&root, "store", r#"{"op":"query","text":"y"}"#);
	let b = lab.call(&root, "store", r#"{"op":"query","text":"y"}"#);
	assert_eq!(a["node"], b["node"]);
	let daemon = lab.daemon(&root);
	assert_ne!(daemon, 0);
	assert_eq!(lab.compositions(&root), 1);
	assert_eq!(lab.nodes(&root), 3);
}

fn alive(pid: u32) -> bool {
	std::process::Command::new("/bin/ps")
		.args(["-p", &pid.to_string()])
		.output()
		.is_ok_and(|o| o.status.success())
}
