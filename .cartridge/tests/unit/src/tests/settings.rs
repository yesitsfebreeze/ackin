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
