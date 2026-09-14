use crate::settings::{apply, defaults, Specs};
use serde_json::json;

fn specs() -> Specs {
	serde_json::from_value(json!({
		"dir": {"type": "string", "default": ".cartridge/memory"},
		"owner": {"type": "table", "default": null, "optional": true},
		"owner.timeout_ms": {"type": "integer", "default": 30000, "min": 1},
	}))
	.unwrap()
}

/// An optional table that nothing configures stays nothing. Filling the keys
/// declared *inside* it would build the table its absence is the whole point
/// of — an unattached memory would arrive holding an owner and read as
/// attached, which is the failure this guards.
#[test]
fn keys_inside_an_absent_optional_table_stay_absent() {
	assert_eq!(defaults(&specs())["owner"], json!(null));
	let settled = apply(&specs(), json!({"dir": "bank"}), "memory").unwrap();
	assert_eq!(settled, json!({"dir": "bank", "owner": null}));
}

/// Naming the table is what brings its inside into being, defaults and all.
#[test]
fn naming_the_table_fills_the_keys_inside_it() {
	let settled = apply(&specs(), json!({"owner": {}}), "memory").unwrap();
	assert_eq!(settled["owner"], json!({"timeout_ms": 30000}));
}

/// The project config is a gated read: an untrusted one is refused where the
/// layer is evaluated, not only in the trust tests.
#[test]
fn an_untrusted_project_config_is_refused_at_its_read() {
	crate::tests::home();
	let dir = tempfile::tempdir().unwrap();
	crate::tests::write(dir.path(), "config.lua", "return {}");
	let refused = crate::settings::read(&dir.path().join("config.lua"))
		.unwrap_err()
		.to_string();
	assert!(refused.contains("is in no trusted project"), "{refused}");
}

/// A configuration file that never returns is refused, not a hang: the state
/// the file runs in has a budget, and the base's own path keeps refusing.
#[test]
fn a_configuration_file_that_never_returns_is_refused() {
	crate::tests::home();
	let dir = tempfile::tempdir().unwrap();
	crate::tests::write(dir.path(), "config.lua", "while true do end");
	crate::trust::record(dir.path()).unwrap();
	let refused = crate::settings::read(&dir.path().join("config.lua"))
		.unwrap_err()
		.to_string();
	assert!(refused.contains("Lua instructions"), "{refused}");
	assert!(refused.contains("config.lua"), "{refused}");
}
