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
		&json!({"name": "provider", "entry": "init.lua", "provide": ["provider"]}).to_string(),
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

/// A ledger root holds the cartridges; no `init.lua` is written, because the
/// one-cartridge ask is the ask that must not need one.
fn tree(dir: &Path, consumer: Value) {
	for name in ["provider", "consumer"] {
		std::fs::create_dir_all(dir.join(name)).unwrap();
	}
	write(
		dir,
		"provider/cartridge.json",
		&json!({"name": "provider", "entry": "init.lua", "provide": ["provider"]}).to_string(),
	);
	write(
		dir,
		"provider/init.lua",
		r#"return { provide = {"provider"}, apply = function(ctx)
			ctx:provide("provider", function() return 41 end)
		end }"#,
	);
	let mut manifest = json!({"name": "consumer", "entry": "init.lua", "needs": ["provider"]});
	for (key, value) in consumer.as_object().unwrap() {
		manifest[key] = value.clone();
	}
	write(dir, "consumer/cartridge.json", &manifest.to_string());
	write(
		dir,
		"consumer/init.lua",
		r#"return { provide = {"consumer.selftest", "consumer.integration"},
		apply = function(ctx)
			ctx:provide("consumer.selftest", function() return true end)
			ctx:provide("consumer.integration", function() return ctx.provider() + 1 == 42 end)
		end }"#,
	);
}

const SOLO_CONTRACTS: fn() -> Value =
	|| json!({"selftest": "consumer.selftest", "integration": "consumer.integration"});

/// One cartridge is verified in isolation: no profile is read, the ledger
/// resolves the need, and the contract runs against the real dependency.
#[tokio::test(flavor = "multi_thread")]
async fn one_cartridge_is_verified_without_a_profile() {
	let dir = tempfile::tempdir().unwrap();
	tree(dir.path(), SOLO_CONTRACTS());
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(
		host.verify_one("consumer").await.unwrap(),
		(2, Vec::new())
	);
}

/// The cartridges a need binds to are loaded to be reached, not graded: the
/// run answers the target's contract, so a provider's own failing contract
/// does not fail a verification of the cartridge being written.
#[tokio::test(flavor = "multi_thread")]
async fn the_providers_a_need_binds_to_are_reached_but_never_graded() {
	let dir = tempfile::tempdir().unwrap();
	tree(dir.path(), SOLO_CONTRACTS());
	write(
		dir.path(),
		"provider/cartridge.json",
		&json!({"name": "provider", "entry": "init.lua", "provide": ["provider", "provider.fail"], "selftest": "provider.fail"})
			.to_string(),
	);
	write(
		dir.path(),
		"provider/init.lua",
		r#"return { provide = {"provider", "provider.fail"}, apply = function(ctx)
			ctx:provide("provider", function() return 41 end)
			ctx:provide("provider.fail", function() return false end)
		end }"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(host.verify_one("consumer").await.unwrap(), (2, Vec::new()));
}

/// A need nothing offers is the wall verify exists to show, named before
/// anything loads — in isolation that is the finding, not an error at run time.
#[tokio::test(flavor = "multi_thread")]
async fn a_need_that_binds_to_nothing_is_named_instead_of_run() {
	let dir = tempfile::tempdir().unwrap();
	tree(dir.path(), json!({"selftest": "consumer.selftest", "needs": ["absent"]}));
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(
		host.verify_one("consumer").await.unwrap_err(),
		"consumer: need `absent` binds to nothing in the tree"
	);
}

/// Two offers of one scope are the ledger's clash, named where the ask is made.
#[tokio::test(flavor = "multi_thread")]
async fn a_clashed_need_stops_the_run_before_it_loads() {
	let dir = tempfile::tempdir().unwrap();
	tree(dir.path(), SOLO_CONTRACTS());
	std::fs::create_dir_all(dir.path().join("twin")).unwrap();
	write(
		dir.path(),
		"twin/cartridge.json",
		&json!({"name": "twin", "entry": "init.lua", "provide": ["provider"]}).to_string(),
	);
	write(dir.path(), "twin/init.lua", "return {}");
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(
		host.verify_one("consumer").await.unwrap_err(),
		"consumer: need `provider` is ambiguous (provider, twin)"
	);
}

/// A nested cartridge is verified by its path from the root, and its need
/// binds outward one scope, to its sibling, exactly the ledger's walk.
#[tokio::test(flavor = "multi_thread")]
async fn a_nested_cartridge_is_verified_by_its_path_from_the_root() {
	let dir = tempfile::tempdir().unwrap();
	std::fs::create_dir_all(dir.path().join("outer/helper")).unwrap();
	std::fs::create_dir_all(dir.path().join("outer/inner")).unwrap();
	// A directory that is not a cartridge is not descended into, so the parent
	// holds a document of its own: the subtree a walk travels through is a
	// chain of cartridges, and a plain folder does not extend it.
	write(
		dir.path(),
		"outer/cartridge.json",
		&json!({"name": "outer", "entry": "init.lua"}).to_string(),
	);
	write(dir.path(), "outer/init.lua", "return {}");
	write(
		dir.path(),
		"outer/helper/cartridge.json",
		&json!({"name": "helper", "entry": "init.lua", "provide": ["helper"]}).to_string(),
	);
	write(
		dir.path(),
		"outer/helper/init.lua",
		r#"return { provide = {"helper"}, apply = function(ctx)
			ctx:provide("helper", function() return 41 end)
		end }"#,
	);
	write(
		dir.path(),
		"outer/inner/cartridge.json",
		&json!({"name": "inner", "entry": "init.lua", "needs": ["helper"],
			"selftest": "inner.check"}).to_string(),
	);
	write(
		dir.path(),
		"outer/inner/init.lua",
		r#"return { provide = {"inner.check"}, apply = function(ctx)
			ctx:provide("inner.check", function() return ctx.helper() == 41 end)
		end }"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(host.verify_one("outer/inner").await.unwrap(), (1, Vec::new()));
}

/// A failing contract is the run's whole answer: the line names the cartridge
/// by its path from the root, the obligation and the key, and nothing passes.
#[tokio::test(flavor = "multi_thread")]
async fn a_failing_contract_names_the_path_the_obligation_and_the_key() {
	let dir = tempfile::tempdir().unwrap();
	tree(
		dir.path(),
		json!({"selftest": "consumer.selftest", "integration": "consumer.integration"}),
	);
	write(
		dir.path(),
		"consumer/init.lua",
		r#"return { provide = {"consumer.selftest", "consumer.integration"}, apply = function(ctx)
			ctx:provide("consumer.selftest", function() return false end)
			ctx:provide("consumer.integration", function() return true end)
		end }"#,
	);
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	assert_eq!(
		host.verify_one("consumer").await.unwrap(),
		(2, vec!["consumer selftest `consumer.selftest` returned false".to_owned()])
	);
}

/// A cartridge that is not in the tree is an error that names the ask, not a
/// run that graded nothing.
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_cartridge_is_named_rather_than_run() {
	let dir = tempfile::tempdir().unwrap();
	tree(dir.path(), SOLO_CONTRACTS());
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	let error = host.verify_one("absent").await.unwrap_err();
	assert!(
		error.starts_with("`absent` is not a cartridge under "),
		"{error}"
	);
}
