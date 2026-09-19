//! Search ranks what the providers found.

use serde_json::json;

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn search_ranks_what_the_providers_found_with_the_fabrics_formula() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	let host = boot(dir.path(), &["files"]).await;
	let answer = host
		.asp(json!({"op": "search", "query": "alpha"}))
		.await
		.unwrap();
	let hits = answer["hits"].as_array().unwrap();
	assert_eq!(hits[0]["node"]["id"], "file:src/alpha.rs", "{answer}");
	assert!(hits[0]["score"].as_f64().unwrap() > 0.0);
	assert_eq!(hits[1]["node"]["id"], "file:src/zeta.rs");
	assert_eq!(hits[1]["why"], "provider match");
	assert_eq!(answer["edges"][0]["kind"], "imports", "{answer}");
	assert_eq!(answer["edges"][0]["contributor"], "files");
	host.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn provider_budget_keeps_fast_evidence_and_reports_the_slow_source() {
	let dir = tempfile::tempdir().unwrap();
	files(dir.path());
	cartridge(
		dir.path(),
		"slow",
		json!({
		 "name":"slow", "entry":"init.lua", "events":{"asp.slow":{}}, "listen":["asp.slow"],
		 "asp":{"schemes":{"file":{}},"search":true}
		}),
		r#"cartridge.listen("asp.slow", function(request)
  local stop = os.clock() + 1
  while os.clock() < stop do end
  return { nodes = { { id = "file:slow" } } }
 end)"#,
	);
	let host = boot(dir.path(), &["files", "slow"]).await;
	for request in [
		json!({"op":"search","query":"alpha"}),
		json!({"op":"expand","entity":"file:src/alpha.rs"}),
	] {
		let mut request = request;
		request["provider_timeout_ms"] = json!(100);
		request["observe"] = json!(false);
		let started = std::time::Instant::now();
		let answer = host.asp(request).await.unwrap();
		assert!(
			started.elapsed() < std::time::Duration::from_millis(800),
			"{answer}"
		);
		let sources = answer["sources"].as_array().unwrap();
		assert_eq!(
			sources
				.iter()
				.find(|s| s["contributor"] == "files")
				.unwrap()["state"],
			"available",
			"{answer}"
		);
		let slow = sources.iter().find(|s| s["contributor"] == "slow").unwrap();
		assert_eq!(slow["state"], "unavailable", "{answer}");
		assert!(
			slow["error"].as_str().unwrap().contains("budget expired"),
			"{answer}"
		);
		assert!(answer.to_string().contains("file:src/alpha.rs"), "{answer}");
		assert!(!answer.to_string().contains("file:slow"), "{answer}");
	}
	for value in [json!(0), json!(-1), json!(1.5), json!(30001), json!("100")] {
		assert!(host
			.asp(json!({"op":"search","query":"alpha","provider_timeout_ms":value}))
			.await
			.is_err());
	}
	host.stop().await;
}
