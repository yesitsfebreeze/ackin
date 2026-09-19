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
		optional: Vec::new(),
		listen: strings(listen),
		asp: Default::default(),
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
		socket2::SockRef::from(&fd)
			.local_addr()
			.unwrap()
			.as_socket()
			.unwrap()
			.port()
	};
	let taken = port(fd);

	let private = Host::new(dir.path(), dir.path().join(".cartridge"))
		.unwrap()
		.private();
	assert_ne!(private.socket_path(), daemon.socket_path());
	let own = private.listener_for("proxy", "127.0.0.1:0").unwrap();
	assert_ne!(port(own), taken, "the daemon's port is not shared");
	assert_eq!(
		private.listener_for("proxy", "127.0.0.1:0").unwrap(),
		own,
		"kept across restarts"
	);
	let memo = std::fs::read_to_string(
		daemon
			.sockets
			.join(format!("{}.port", crate::host::socket::file_name("proxy"))),
	)
	.unwrap();
	assert!(
		memo.ends_with(&format!(":{taken}")),
		"the project's memo stays the daemon's: {memo}"
	);
}

fn need_fixture(dir: &std::path::Path, id: &str, manifest: serde_json::Value, code: &str) {
	crate::tests::write(dir, &format!("{id}/cartridge.json"), &manifest.to_string());
	crate::tests::write(dir, &format!("{id}/init.lua"), code);
}

async fn need_host(dir: &std::path::Path, ids: &[&str]) -> (Arc<Host>, Result<()>) {
	static BIN: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
	let bin = BIN.get_or_init(|| crate::tests::built(&["--bin", "cartridge"]));
	std::env::set_var(crate::host::NODE_BIN_ENV, bin);
	let entries: Vec<_> = ids
		.iter()
		.map(|id| format!("{{id={id:?}, path={id:?}}}"))
		.collect();
	crate::tests::write(
		dir,
		".cartridge/init.lua",
		&format!("return {{{}}}", entries.join(",")),
	);
	crate::tests::home();
	crate::trust::record(dir).unwrap();
	let host = Host::new(dir, dir.join(".cartridge")).unwrap();
	let result = host.reconcile().await;
	(host, result)
}

#[tokio::test(flavor = "multi_thread")]
async fn optional_need_without_provider_loads_and_answers() {
	use crate::host::State;
	use serde_json::json;

	for provided in [false, true] {
		let dir = tempfile::tempdir().unwrap();
		need_fixture(
			dir.path(),
			"consumer",
			json!({
				"name":"consumer", "entry":"init.lua", "events":{"answer":{}},
				"listen":["answer"], "needs":["tool.missing?"]
			}),
			if provided {
				r#"cartridge.listen("answer", function() return cartridge.bail("tool.missing", {}) end)"#
			} else {
				r#"cartridge.listen("answer", function() return "without provider" end)"#
			},
		);
		need_fixture(
			dir.path(),
			"hard",
			json!({
				"name":"hard", "entry":"init.lua", "needs":["tool.missing"]
			}),
			"",
		);
		need_fixture(
			dir.path(),
			"stray",
			json!({
				"name":"stray", "entry":"init.lua", "listen":["nobody.declares"],
				"needs":["nobody.declares?"]
			}),
			"",
		);
		let mut ids = vec!["consumer", "hard", "stray"];
		if provided {
			need_fixture(
				dir.path(),
				"provider",
				json!({
					"name":"provider", "entry":"init.lua", "events":{"tool.missing":{}},
					"listen":["tool.missing"]
				}),
				r#"cartridge.listen("tool.missing", function() return "with provider" end)"#,
			);
			ids.push("provider");
		}
		let (host, reconciled) = need_host(dir.path(), &ids).await;
		let statuses = host.status();
		let answer = host.bail("answer", json!(null)).await;
		host.stop().await;
		reconciled.unwrap();
		let status = |id: &str| statuses.iter().find(|s| s.id == id).unwrap();
		assert_eq!(
			status("consumer").state,
			State::Active,
			"{:?}",
			status("consumer").error
		);
		assert_eq!(
			answer.unwrap(),
			Some(json!(if provided {
				"with provider"
			} else {
				"without provider"
			}))
		);
		assert_eq!(status("stray").state, State::Failed);
		assert!(status("stray")
			.error
			.as_deref()
			.unwrap()
			.contains("listens to `nobody.declares`, which no cartridge declares"));
		if provided {
			assert_eq!(status("hard").state, State::Active);
		} else {
			assert_eq!(status("hard").state, State::Failed);
			assert!(status("hard")
				.error
				.as_deref()
				.unwrap()
				.contains("needs `tool.missing`, which no cartridge declares"));
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn optional_glob_is_refused_without_losing_optionality() {
	use crate::host::State;
	use serde_json::json;

	for provided in [false, true] {
		let dir = tempfile::tempdir().unwrap();
		need_fixture(
			dir.path(),
			"consumer",
			json!({
				"name":"consumer", "entry":"init.lua", "needs":["fixture.*?"]
			}),
			"",
		);
		let mut ids = vec!["consumer"];
		if provided {
			need_fixture(
				dir.path(),
				"provider",
				json!({
					"name":"provider", "entry":"init.lua", "events":{"fixture.answer":{}},
					"listen":["fixture.answer"]
				}),
				r#"cartridge.listen("fixture.answer", function() return 42 end)"#,
			);
			ids.push("provider");
		}
		let (host, reconciled) = need_host(dir.path(), &ids).await;
		let statuses = host.status();
		host.stop().await;
		reconciled.unwrap();
		let consumer = statuses.iter().find(|s| s.id == "consumer").unwrap();
		assert_eq!(
			consumer.state,
			State::Failed,
			"optional glob accepted (provided={provided})"
		);
		assert!(
			consumer
				.error
				.as_deref()
				.unwrap()
				.contains("optional need `fixture.*?` must be a nonempty exact event name"),
			"{:?}",
			consumer.error
		);
	}
}
