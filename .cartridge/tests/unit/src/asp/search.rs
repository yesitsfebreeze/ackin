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
