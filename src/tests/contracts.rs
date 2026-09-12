//! Registration contracts: a manifest names a `selftest` and an `integration`
//! key the cartridge already provides, and `verify` calls them after apply.

use super::*;
use crate::runtime::Runtime;
use serde_json::json;

/// A provider cartridge plus a consumer whose integration check reaches the
/// provider's real service. `contract` is the consumer's manifest declaration.
fn profile(dir: &Path, contract: Value, selftest: &str, integration: &str) {
	for name in ["provider", "consumer"] {
		std::fs::create_dir_all(dir.join(name)).unwrap();
	}
	write(
		dir,
		"provider/cartridge.json",
		&json!({"name": "provider", "entry": "init.lua"}).to_string(),
	);
	write(
		dir,
		"provider/init.lua",
		r#"return { provide = {"provider"}, apply = function(ctx)
			ctx:provide("provider", function() return 41 end)
		end }"#,
	);
	let mut manifest = json!({"name": "consumer", "entry": "init.lua"});
	for (key, value) in contract.as_object().unwrap() {
		manifest[key] = value.clone();
	}
	write(dir, "consumer/cartridge.json", &manifest.to_string());
	write(
		dir,
		"consumer/init.lua",
		&format!(
			r#"return {{ inject = {{"provider"}}, provide = {{"consumer.selftest", "consumer.integration"}},
			apply = function(ctx)
				ctx:provide("consumer.selftest", function() return {selftest} end)
				ctx:provide("consumer.integration", function() return {integration} end)
			end }}"#
		),
	);
	write(
		dir,
		"init.lua",
		r#"return { { id = "provider", path = "provider" }, { id = "consumer", path = "consumer" } }"#,
	);
}

const DECLARED: fn() -> Value =
	|| json!({"selftest": "consumer.selftest", "integration": "consumer.integration"});

/// The integration check is a real call into the provider cartridge's service.
const WIRED: &str = "ctx.provider() + 1 == 42";

#[tokio::test(flavor = "multi_thread")]
async fn declared_contracts_run_after_apply_and_pass() {
	let dir = tempfile::tempdir().unwrap();
	profile(dir.path(), DECLARED(), "true", WIRED);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(host.verify().await.unwrap(), (2, Vec::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_selftest_fails_closed_and_names_the_cartridge() {
	let dir = tempfile::tempdir().unwrap();
	profile(dir.path(), DECLARED(), "false", WIRED);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let (ran, failures) = host.verify().await.unwrap();
	assert_eq!(ran, 2);
	assert_eq!(
		failures,
		vec!["consumer selftest `consumer.selftest` returned false"]
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_integration_check_fails_closed_with_its_error() {
	let dir = tempfile::tempdir().unwrap();
	profile(dir.path(), DECLARED(), "true", "ctx.absent()");
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let (_, failures) = host.verify().await.unwrap();
	assert_eq!(failures.len(), 1, "{failures:?}");
	assert!(
		failures[0].starts_with("consumer integration `consumer.integration`: "),
		"{failures:?}"
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cartridge_declaring_nothing_is_never_called() {
	let dir = tempfile::tempdir().unwrap();
	profile(dir.path(), json!({}), "false", "ctx.absent()");
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(host.verify().await.unwrap(), (0, Vec::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn one_obligation_may_be_declared_alone() {
	let dir = tempfile::tempdir().unwrap();
	profile(
		dir.path(),
		json!({"selftest": "consumer.selftest"}),
		"true",
		"ctx.absent()",
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(host.verify().await.unwrap(), (1, Vec::new()));
}
