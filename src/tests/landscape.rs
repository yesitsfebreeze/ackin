use super::{boot, settle, write};
use serde_json::{json, Value};

fn entry<'a>(landscape: &'a Value, id: &str) -> &'a Value {
	landscape["entries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"] == id)
		.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn landscape_distinguishes_provider_states_and_refreshes_committed_generations() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"good.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",42) end}"#,
	);
	write(
		dir.path(),
		"consumer.lua",
		r#"return {inject={"answer"},apply=function() end}"#,
	);
	write(
		dir.path(),
		"waiting.lua",
		r#"return {inject={"absent"},apply=function() end}"#,
	);
	write(dir.path(), "bad.lua", "error('bad entry')");
	write(dir.path(), "disabled.lua", "error('must not evaluate')");
	write(
		dir.path(),
		"init.lua",
		r#"return {
		{id="good",path="good.lua",config={secret="never print me"}},
		{id="consumer",path="consumer.lua"},{id="waiting",path="waiting.lua"},
		{id="bad",path="bad.lua"},{id="missing",path="missing.lua"},
		{id="disabled",path="disabled.lua",disabled=true}}
	"#,
	);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("good").unwrap().settled().await;
	host.fiber_of("consumer").unwrap().settled().await;
	let first = host.landscape();
	assert_eq!(first["profile"], json!(dir.path().canonicalize().unwrap()));
	assert!(!first.to_string().contains("never print me"));
	assert_eq!(entry(&first, "good")["state"], "Active");
	assert_eq!(
		entry(&first, "consumer")["dependencies"][0]["provider"],
		"good"
	);
	assert_eq!(entry(&first, "waiting")["state"], "Inactive");
	assert!(entry(&first, "waiting")["dependencies"][0]["provider"].is_null());
	assert_eq!(entry(&first, "bad")["state"], "Failed");
	assert!(entry(&first, "bad")["error"]
		.as_str()
		.unwrap()
		.contains("bad entry"));
	assert_eq!(entry(&first, "missing")["state"], "Missing");
	assert_eq!(entry(&first, "disabled")["state"], "Disabled");
	assert_eq!(entry(&first, "disabled")["declarations_available"], false);
	assert!(entry(&first, "disabled")["error"].is_null());

	write(dir.path(), "good.lua", "this is not valid lua");
	host.replace(&dir.path().join("good.lua")).await;
	let failed = host.landscape();
	assert_eq!(
		entry(&failed, "good")["generation"],
		entry(&first, "good")["generation"]
	);
	assert_eq!(entry(&failed, "good")["state"], "Active");
	assert!(entry(&failed, "good")["error"].is_string());
	assert_eq!(entry(&failed, "good")["sources"][0]["changed"], true);
	write(
		dir.path(),
		"good.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",12345) end}"#,
	);
	host.replace(&dir.path().join("good.lua")).await;
	host.fiber_of("good").unwrap().settled().await;
	let current = host.landscape();
	assert_ne!(
		entry(&current, "good")["generation"],
		entry(&first, "good")["generation"]
	);
	assert!(entry(&current, "good")["error"].is_null());
	assert_eq!(entry(&current, "good")["sources"][0]["changed"], false);
	for id in ["good", "consumer", "waiting"] {
		host.fiber_of(id).unwrap().dispose().await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn landscape_lists_dynamic_children_and_tracked_source_paths() {
	let dir = tempfile::tempdir().unwrap();
	write(
		dir.path(),
		"child.lua",
		r#"return {provide={"answer"},apply=function(ctx) ctx:provide("answer",1) end}"#,
	);
	write(
		dir.path(),
		"parent.lua",
		r#"return {apply=function(ctx) ctx:cartridge("child.lua") end}"#,
	);
	write(
		dir.path(),
		"consumer.lua",
		r#"return {inject={"answer"},apply=function() end}"#,
	);
	write(
		dir.path(),
		"init.lua",
		r#"return {{id="parent",path="parent.lua"},{id="consumer",path="consumer.lua"}}"#,
	);
	let run = |args: &[&str]| {
		let output = std::process::Command::new("git")
			.args(args)
			.current_dir(dir.path())
			.output()
			.unwrap();
		assert!(
			output.status.success(),
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);
	};
	run(&["init"]);
	run(&["add", "."]);
	let (host, _) = boot(dir.path()).await;
	host.fiber_of("parent").unwrap().settled().await;
	settle().await;
	let landscape = host.landscape();
	let child = landscape["entries"]
		.as_array()
		.unwrap()
		.iter()
		.find(|row| row["id"] == "child")
		.unwrap();
	assert_eq!(child["dynamic"], true);
	assert_eq!(child["parent"], "parent");
	assert_eq!(child["provide"], json!(["answer"]));
	assert_eq!(child["state"], "Active");
	assert_eq!(
		entry(&landscape, "consumer")["dependencies"][0]["provider"],
		"child"
	);
	let consumer = entry(&landscape, "consumer");
	assert_eq!(consumer["dependencies"][0]["key"], "answer");
	assert!(entry(&landscape, "parent").get("tracked").is_none());
	host.fiber_of("parent").unwrap().dispose().await;
	host.fiber_of("consumer").unwrap().dispose().await;
}
