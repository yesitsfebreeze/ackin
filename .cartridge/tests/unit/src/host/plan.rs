use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use super::*;

fn plan(id: &str, events: &[(&str, Option<u64>)], needs: &[&str], listen: &[&str]) -> Arc<Plan> {
	let strings = |names: &[&str]| names.iter().map(|n| n.to_string()).collect();
	Arc::new(Plan {
		id: id.into(),
		name: id.into(),
		root: PathBuf::new(),
		entry: PathBuf::new(),
		entry_sha256: String::new(),
		events: events
			.iter()
			.map(|(name, timeout_ms)| {
				let event = Event {
					timeout_ms: *timeout_ms,
					..Event::default()
				};
				(name.to_string(), event)
			})
			.collect(),
		needs: strings(needs),
		listen: strings(listen),
		config: serde_json::Value::Null,
		grant: Grant::default(),
		sources: Vec::new(),
		listener: None,
	})
}

fn composition() -> (Arc<Host>, Vec<Arc<Plan>>, BTreeMap<String, Directory>) {
	let dir = tempfile::tempdir().unwrap();
	let host = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	let plans = vec![
		plan("a", &[("a.ask", Some(5))], &[], &["a.ask"]),
		plan("b", &[("b.news", None)], &["a.ask"], &[]),
		plan("c", &[], &[], &["a.ask"]),
		plan("d", &[], &[], &["b.news"]),
	];
	let (catalogue, clashes) = catalogue(&plans);
	assert!(clashes.is_empty());
	let directories = plans
		.iter()
		.map(|p| (p.id.clone(), directory(&host, p, &plans, &catalogue)))
		.collect();
	(host, plans, directories)
}

fn tokens(directory: &Directory) -> HashSet<String> {
	directory
		.events
		.values()
		.flat_map(|e| e.listeners.iter().map(|l| l.token.clone()))
		.collect()
}

#[test]
fn a_cartridge_gets_listeners_only_for_what_it_defines_or_needs() {
	let (_host, _plans, dirs) = composition();
	assert_eq!(dirs["b"].sends, vec!["a.ask", "b.news"]);
	let asked: Vec<&str> = dirs["b"].events["a.ask"]
		.listeners
		.iter()
		.map(|l| l.cartridge.as_str())
		.collect();
	assert_eq!(asked, vec!["a", "c"]);
	assert!(dirs["d"].sends.is_empty());
	assert!(tokens(&dirs["d"]).is_empty(), "d may send nothing");
	assert!(dirs["c"].events["a.ask"].listeners.is_empty());
	assert_eq!(dirs["d"].events["a.ask"].owner, "a");
}

#[test]
fn a_listener_accepts_each_declared_sender_for_its_events_only() {
	let (host, _plans, dirs) = composition();
	let accept = &dirs["c"].accept;
	assert_eq!(accept.len(), 2, "{accept:?}");
	assert_eq!(accept[&host.token("b", Some("c"))].events, vec!["a.ask"]);
	assert_eq!(accept[&host.token("a", Some("c"))].from, "a");
	assert_eq!(
		dirs["d"].accept[&host.token("b", Some("d"))].events,
		vec!["b.news"]
	);
	assert!(dirs["b"].accept.is_empty(), "nobody may send b anything");
}

#[test]
fn no_node_holds_a_token_another_node_accepts_from_someone_else() {
	let (_host, plans, dirs) = composition();
	for holder in &plans {
		let held = tokens(&dirs[&holder.id]);
		for other in &plans {
			for (token, accept) in &dirs[&other.id].accept {
				if held.contains(token) {
					assert_eq!(
						accept.from, holder.id,
						"{} holds {}'s token",
						holder.id, accept.from
					);
				}
			}
		}
	}
}

#[test]
fn every_event_carries_its_deadline() {
	let (_host, _plans, dirs) = composition();
	assert_eq!(dirs["b"].events["a.ask"].timeout_ms, 5);
	assert_eq!(
		dirs["b"].events["b.news"].timeout_ms,
		crate::settings::host().event_timeout_ms
	);
}

/// `run` and `verify` compose beside the daemon. Their nodes must reach their
/// own host, not the daemon's socket (which rejects their tokens), and their
/// listeners must not share the daemon's remembered port.
#[cfg(unix)]
#[test]
fn a_private_host_keeps_its_socket_and_ports_to_itself() {
	let dir = tempfile::tempdir().unwrap();
	let daemon = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap();
	let fd = daemon.listener_for("proxy", "127.0.0.1:0").unwrap();
	let port = |fd| {
		use std::os::fd::BorrowedFd;
		let fd = unsafe { BorrowedFd::borrow_raw(fd) };
		socket2::SockRef::from(&fd).local_addr().unwrap().as_socket().unwrap().port()
	};
	let taken = port(fd);

	let private = Host::new(dir.path(), dir.path().join(".cartridge")).unwrap().private();
	assert_ne!(private.socket_path(), daemon.socket_path());
	let own = private.listener_for("proxy", "127.0.0.1:0").unwrap();
	assert_ne!(port(own), taken, "the daemon's port is not shared");
	assert_eq!(private.listener_for("proxy", "127.0.0.1:0").unwrap(), own, "kept across restarts");
	let memo = std::fs::read_to_string(daemon.sockets.join(format!("{}.port", crate::host::socket::file_name("proxy")))).unwrap();
	assert!(memo.ends_with(&format!(":{taken}")), "the project's memo stays the daemon's: {memo}");
}
