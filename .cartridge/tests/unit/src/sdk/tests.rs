use super::*;
fn host() -> (Host, mpsc::UnboundedReceiver<Option<Value>>) {
	let (tx, rx) = mpsc::unbounded_channel();
	(
		Host {
			version_queries: false,
			reload: Arc::default(),
			link: Link::new(tx, "gone"),
			events: Arc::default(),
			services: Arc::default(),
			streams: Arc::default(),
			finalizers: Arc::default(),
			declared: vec![],
			children: Arc::default(),
		},
		rx,
	)
}
#[tokio::test]
async fn reload_registration_dispatches_boolean_values_for_prepare_and_cancel() {
	let (host, mut rx) = host();
	host.on_reload(|value: Value| async move {
		Ok(json!({"prepare":value.as_bool().expect("boolean Value")}))
	});
	for prepare in [true, false] {
		let request = json!({"id":7,"reload":prepare});
		host.dispatch(&host.reload, &request, "reload", request["reload"].clone());
		let reply = rx.recv().await.unwrap().unwrap();
		assert_eq!(reply["reply"], 7);
		assert_eq!(reply["data"]["prepare"], prepare);
	}
}
#[tokio::test]
async fn cancelling_nested_startup_kills_and_reaps_its_child() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("child.sh");
	let pid = dir.path().join("pid");
	std::fs::write(&path,format!("#!/bin/sh\nif [ \"$1\" = hello ]; then echo '{{}}'; exit 0; fi\necho $$ > '{}'\nwhile read line; do :; done\n",pid.display())).unwrap();
	std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
	let (host, _rx) = host();
	let task = tokio::spawn(async move {
		host.spawn("nested", &[path.display().to_string()], Value::Null)
			.await
	});
	let pid = tokio::time::timeout(std::time::Duration::from_secs(3), async {
		loop {
			if let Some(child) = std::fs::read_to_string(&pid)
				.ok()
				.and_then(|value| value.trim().parse::<u32>().ok())
				.filter(|pid| *pid > 0)
			{
				break child;
			}
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
	task.abort();
	assert!(task.await.unwrap_err().is_cancelled());
	let pid = pid.to_string();
	tokio::time::timeout(std::time::Duration::from_secs(3), async {
		while std::process::Command::new("/bin/kill")
			.args(["-0", pid.trim()])
			.stderr(std::process::Stdio::null())
			.status()
			.unwrap()
			.success()
		{
			tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		}
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn slow_stream_handlers_have_bounded_queues_and_receive_a_gap() {
	let (host, mut wire) = host();
	let (tx, mut rx) = mpsc::channel(WATCHER_EVENTS);
	host.streams.lock().insert("slow".into(), tx);
	for seq in 1..=1000 {
		host.deliver_stream("slow", json!({"seq":seq,"kind":"data"}));
	}
	assert!(!host.streams.lock().contains_key("slow"));
	assert_eq!(wire.recv().await.unwrap().unwrap()["unsubscribe"], "slow");
	let mut seen = Vec::new();
	while let Some(value) = rx.recv().await {
		seen.push(value);
	}
	assert_eq!(seen.len(), WATCHER_EVENTS);
	assert_eq!(seen.last().unwrap()["kind"], "error");
}

#[tokio::test]
async fn older_hosts_use_uncached_descriptions_without_an_unknown_wire_request() {
	let (host, mut wire) = host();
	assert_eq!(
		host.service_versions(&["tool.read".into()]).await.unwrap(),
		Value::Null
	);
	assert!(wire.try_recv().is_err());
}
