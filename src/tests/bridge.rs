use super::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn bridge_status_tracks_active_generations_and_scopes_backend_calls() {
	let dir = tempfile::tempdir().unwrap();
	std::fs::create_dir(dir.path().join("p")).unwrap();
	write(
		dir.path(),
		"p/cartridge.json",
		r#"{"name":"p","entry":"init.lua","ui":"ui.tsx"}"#,
	);
	write(dir.path(), "p/ui.tsx", "export function setup() {}");
	write(
		dir.path(),
		"p/init.lua",
		r#"return {provide={"counter"},apply=function(ctx) ctx:provide("counter",function(args) return args end) end}"#,
	);
	write(
		dir.path(),
		"headless.lua",
		r#"return {provide={"other"},apply=function(ctx) ctx:provide("other",42) end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="p",path="p"},{id="headless",path="headless.lua"}}"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("p").unwrap().settled().await;
	let first = host.bridge_status();
	assert_eq!(first.as_array().unwrap().len(), 1);
	assert_eq!(first[0]["id"], "p");
	let generation = first[0]["generation"].as_u64().unwrap();
	assert_eq!(
		host
			.bridge_call("p", generation, "counter", json!(3))
			.await
			.unwrap(),
		json!(3)
	);
	assert!(host
		.bridge_call("p", generation, "other", json!(null))
		.await
		.is_err());
	assert!(!host.bridge_enabled(generation));

	write(
		dir.path(),
		"p/ui.tsx",
		"export function setup() { /* version two */ }",
	);
	host.replace(&dir.path().join("p/ui.tsx")).await;
	let second = host.bridge_status();
	assert_ne!(second[0]["generation"], first[0]["generation"]);
	assert!(host
		.bridge_call("p", generation, "counter", json!(3))
		.await
		.is_err());
	write(dir.path(), "p/init.lua", "invalid Lua");
	host.replace(&dir.path().join("p/init.lua")).await;
	assert_eq!(host.bridge_status(), second);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="headless",path="headless.lua"}}"#,
	);
	host.reconcile().await.unwrap();
	assert_eq!(host.bridge_status(), json!([]));
}

#[test]
fn bridge_manifest_rejects_paths_outside_the_cartridge() {
	let dir = tempfile::tempdir().unwrap();
	std::fs::create_dir(dir.path().join("p")).unwrap();
	write(dir.path(), "p/init.lua", "return {apply=function() end}");
	for ui in [
		"../outside.tsx",
		"/absolute.tsx",
		"",
		"ui.lua",
		"missing.tsx",
	] {
		write(
			dir.path(),
			"p/cartridge.json",
			&json!({"name":"p","entry":"init.lua","ui":ui}).to_string(),
		);
		assert!(crate::loader::Cartridge::read(&dir.path().join("p/cartridge.json")).is_err());
	}
}

#[test]
fn process_scripts_resolve_relative_to_the_cartridge() {
	use std::os::unix::fs::PermissionsExt;
	let dir = tempfile::tempdir().unwrap();
	let script = dir.path().join("run");
	std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
	std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
	assert_eq!(
		crate::cartridge::executable("./run", dir.path()).unwrap(),
		script.canonicalize().unwrap()
	);
}
