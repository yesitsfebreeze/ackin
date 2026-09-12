use super::*;
use serde_json::json;

fn package(dir: &Path, name: &str, entry: &str) {
	std::fs::create_dir_all(dir.join("p")).unwrap();
	write(
		dir,
		"p/cartridge.json",
		&json!({"name": name, "entry": entry}).to_string(),
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn folder_manifest_names_the_component_and_reloads_its_entry() {
	let dir = tempfile::tempdir().unwrap();
	package(dir.path(), "declared-name", "first.lua");
	write(
		dir.path(),
		"p/first.lua",
		r#"return {apply=function(ctx) ctx:send("version", 1) end}"#,
	);
	write(
		dir.path(),
		"p/second.lua",
		r#"return {apply=function(ctx) ctx:send("version", 2) end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="instance",path="p"}}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(1));
	for path in ["p", "p/cartridge.json"] {
		let component = host.component(&dir.path().join(path), json!({})).unwrap();
		assert_eq!(component.name, "declared-name", "{path}");
	}
	let first = host.fiber_of("instance").unwrap().uid();

	package(dir.path(), "declared-name", "second.lua");
	host.replace(&dir.path().join("p/cartridge.json")).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(2));
	assert_ne!(host.fiber_of("instance").unwrap().uid(), first);

	write(
		dir.path(),
		"p/second.lua",
		r#"return {apply=function(ctx) ctx:send("version", 3) end}"#,
	);
	host.replace(&dir.path().join("p/second.lua")).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(3));
	let current = host.fiber_of("instance").unwrap().uid();
	write(dir.path(), "p/cartridge.json", "broken json");
	host.replace(&dir.path().join("p/cartridge.json")).await;
	assert_eq!(host.fiber_of("instance").unwrap().uid(), current);
	let error = rx.recv().await.unwrap();
	assert!(error["error"]["message"]
		.as_str()
		.unwrap()
		.contains("cartridge.json"));
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_manifests_fail_without_evaluating_entries() {
	let dir = tempfile::tempdir().unwrap();
	package(dir.path(), "valid", "init.lua");
	write(dir.path(), "p/init.lua", "error('entry evaluated')");
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	for manifest in [
		json!({"entry":"init.lua"}),
		json!({"name":"valid"}),
		json!({"name":" ","entry":"init.lua"}),
		json!({"name":"valid","entry":"../outside.lua"}),
		json!({"name":"valid","entry":"/absolute.lua"}),
		json!({"name":"valid","entry":""}),
		json!({"name":"valid","entry":"program"}),
	] {
		write(dir.path(), "p/cartridge.json", &manifest.to_string());
		let error = host
			.component(&dir.path().join("p"), json!({}))
			.err()
			.unwrap()
			.to_string();
		assert!(!error.contains("entry evaluated"), "{error}");
		assert!(error.contains("cartridge.json"), "{error}");
	}
	std::fs::remove_file(dir.path().join("p/cartridge.json")).unwrap();
	assert!(host.component(&dir.path().join("p"), json!({})).is_err());
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="disabled",path="p",disabled=true}}"#,
	);
	host.reconcile().await.unwrap();
	assert!(host.manifest().unwrap()[0].error.is_none());
	assert!(host.fiber_of("disabled").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn watcher_reloads_a_folder_when_its_manifest_changes() {
	let dir = tempfile::tempdir().unwrap();
	package(dir.path(), "watched", "first.lua");
	for (file, version) in [("first.lua", 1), ("second.lua", 2)] {
		write(
			dir.path(),
			&format!("p/{file}"),
			&format!("return {{apply=function(ctx) ctx:send('version', {version}) end}}"),
		);
	}
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p/cartridge.json"}}"#,
	);
	let (host, mut rx) = boot(dir.path()).await;
	assert_eq!(next_event(&mut rx, "version").await, json!(1));
	host.watch().unwrap();
	package(dir.path(), "watched", "second.lua");
	assert_eq!(next_event(&mut rx, "version").await, json!(2));
}
