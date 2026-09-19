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

#[tokio::test(flavor = "multi_thread")]
async fn asp_observation_reaches_writer_with_stable_top_level_timestamp() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"writer",
		json!({"name":"writer","entry":"init.lua","events":{"trace":{"description":"writer","schema":{"type":"object"}},"trace_status":{"description":"inspect","schema":{"type":"object"}}},"listen":["trace","trace_status"]}),
		r#"
 local observed = {}
 cartridge.listen("trace", function(args)
  if args.activity and args.activity.kind == "asp" then observed = args end
  return {saved=true}
 end)
 cartridge.listen("trace_status", function() return observed end)
 "#,
	);
	let host = boot(dir.path(), &["files", "writer"]).await;
	let served = tokio::spawn(crate::host::socket::serve(host.clone()));
	host.asp(json!({"op":"expand","entity":"file:src/a.rs"}))
		.await
		.unwrap();
	let saved = host.bail("trace_status", json!({})).await.unwrap().unwrap();
	assert!(saved["ts"].as_u64().unwrap() > 0);
	assert_eq!(saved["ts"], saved["activity"]["ts"]);
	assert_eq!(saved["activity"]["operation"], "expand");
	host.bail("trace",json!({"action":"append","ts":111,"activity":{"kind":"asp","operation":"direct","request":"x".repeat(100000)}})).await.unwrap();
	let direct = host.bail("trace_status", json!({})).await.unwrap().unwrap();
	assert!(direct.to_string().len() < 65536);
	assert_eq!(direct["ts"], 111);
	assert_eq!(direct["activity"]["writer_truncation"]["truncated"], true);

	host.stop().await;
	served.abort();
}
