//! Types and facts come and go with the cartridges that declare them.

use serde_json::{json, Value};

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn unloading_a_provider_removes_its_types_and_facts_and_keeps_the_rest() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	search(dir.path());
	let host = boot(dir.path(), &["files", "search"]).await;

	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert_eq!(types["schemes"]["file"]["owner"], "files");
	assert_eq!(
		types["schemes"]["file"]["contributors"],
		json!(["files", "search"])
	);
	assert_eq!(types["schemes"]["range"]["owner"], "search");
	assert_eq!(types["edges"]["matches"], json!(["search"]));

	let both = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&both), ["file:src/a.rs", "range:src/a.rs:4:1-4:9"]);
	assert_eq!(both["edges"][0]["kind"], "matches");
	assert_eq!(both["edges"][0]["contributor"], "search");
	assert_eq!(both["nodes"][0]["attributes"]["files.bytes"], 12);

	descriptor(dir.path(), &["files"]);
	host.reconcile().await.unwrap();

	let types = host.asp(json!({"op": "types"})).await.unwrap();
	assert!(types["schemes"].get("range").is_none(), "{types}");
	assert!(types["edges"].get("matches").is_none(), "{types}");
	let alone = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&alone), ["file:src/a.rs"]);
	assert_eq!(alone["edges"], json!([]));
	let refused = host
		.asp(json!({"op": "expand", "entity": "range:src/a.rs:4:1-4:9"}))
		.await
		.unwrap_err();
	assert!(
		refused.contains("no loaded cartridge declares"),
		"{refused}"
	);
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_two_providers_assert_survives_one_of_them() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"mirror",
		json!({
			"name": "mirror", "entry": "init.lua",
			"events": {"asp.mirror": {}}, "listen": ["asp.mirror"],
			"asp": {"schemes": {"file": {}}},
		}),
		r#"cartridge.listen("asp.mirror", function(request) return { nodes = { { id = request.entity, name = "mirrored" } } } end)"#,
	);
	let host = boot(dir.path(), &["files", "mirror"]).await;
	let contributors = |answer: &Value| -> Vec<String> {
		answer["nodes"][0]["contributors"]
			.as_array()
			.unwrap()
			.iter()
			.map(|c| c["contributor"].as_str().unwrap().to_owned())
			.collect()
	};
	let both = expand(&host, "file:src/a.rs").await;
	assert_eq!(contributors(&both), ["files", "mirror"]);
	assert_eq!(
		both["nodes"][0]["name"], "a.rs",
		"the owner names the entity"
	);

	descriptor(dir.path(), &["mirror"]);
	host.reconcile().await.unwrap();
	let one = expand(&host, "file:src/a.rs").await;
	assert_eq!(ids(&one), ["file:src/a.rs"]);
	assert_eq!(contributors(&one), ["mirror"]);
	host.stop().await;
}
