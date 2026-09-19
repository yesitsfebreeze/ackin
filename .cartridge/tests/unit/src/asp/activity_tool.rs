use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn agent_activity_schema_and_monitoring_preserve_observed_uses() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	let host = boot(dir.path(), &["files"]).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	let description = host.asp_tool(json!({"op":"describe"})).await.unwrap();
	let schema = jsonschema::validator_for(&description["input_schema"]).unwrap();
	assert!(schema.is_valid(&json!({"op":"activity"})));
	assert!(schema.is_valid(&json!({"op":"expand", "entity":"file:src/a.rs", "observe":false})));
	assert!(!schema.is_valid(&json!({"op":"expand", "observe":"false"})));
	let properties = &description["input_schema"]["properties"];
	assert!(properties["op"]["enum"]
		.as_array()
		.unwrap()
		.contains(&json!("activity")));
	assert_eq!(properties["observe"]["type"], "boolean");
	assert_eq!(properties["observe"]["default"], true);
	let snapshot = || async {
		let answer = host
			.asp_tool(json!({"op":"call", "input":{"op":"activity"}}))
			.await
			.unwrap();
		assert_eq!(answer["error"], false);
		serde_json::from_str::<Value>(answer["content"].as_str().unwrap()).unwrap()
	};
	let before = snapshot().await;
	let answer = host
		.asp_tool(json!({"op":"call", "input":{
			"op":"expand", "entity":"file:src/a.rs", "observe":false
		}}))
		.await
		.unwrap();
	assert_eq!(answer["error"], false);
	assert_eq!(snapshot().await, before);
	host.asp_tool(json!({"op":"call", "input":{
		"op":"expand", "entity":"file:src/a.rs"
	}}))
	.await
	.unwrap();
	let after = snapshot().await;
	assert_eq!(
		after["records"].as_array().unwrap().len(),
		before["records"].as_array().unwrap().len() + 1
	);
	assert_eq!(
		after["records"].as_array().unwrap().last().unwrap()["entities"],
		json!(["file:src/a.rs"])
	);
	assert_eq!(snapshot().await, after);
	host.stop().await;
	served.abort();
}
