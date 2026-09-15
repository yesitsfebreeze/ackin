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

#[test]
fn keys_inside_an_absent_optional_table_stay_absent() {
	assert_eq!(defaults(&specs())["owner"], json!(null));
	let settled = apply(&specs(), json!({"dir": "bank"}), "memory").unwrap();
	assert_eq!(settled, json!({"dir": "bank", "owner": null}));
}

#[test]
fn naming_the_table_fills_the_keys_inside_it() {
	let settled = apply(&specs(), json!({"owner": {}}), "memory").unwrap();
	assert_eq!(settled["owner"], json!({"timeout_ms": 30000}));
}

#[test]
fn yolo_overrides_the_configured_value_only_where_the_cartridge_declares_it() {
	let automatic: Specs = serde_json::from_value(json!({
		"yolo": {"type": "boolean", "default": false},
		"dir": {"type": "string", "default": "."},
	}))
	.unwrap();
	// The caller's environment is not under test: a yolo-mode shell exports
	// CARTRIDGE_YOLO=1, which would flip the very first assertion.
	let ambient = std::env::var_os(crate::settings::YOLO_ENV);
	std::env::remove_var(crate::settings::YOLO_ENV);
	assert_eq!(
		apply(&automatic, json!({"yolo": false}), "agent").unwrap()["yolo"],
		json!(false)
	);
	std::env::set_var(crate::settings::YOLO_ENV, "1");
	let settled = apply(&automatic, json!({"yolo": false}), "agent").unwrap();
	match ambient {
		Some(value) => std::env::set_var(crate::settings::YOLO_ENV, value),
		None => std::env::remove_var(crate::settings::YOLO_ENV),
	}
	assert_eq!(settled["yolo"], json!(true));
	assert_eq!(settled["dir"], json!("."));
	assert!(apply(&specs(), json!({}), "memory")
		.unwrap()
		.get("yolo")
		.is_none());
}

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

#[test]
fn refused_settings_do_not_wedge_the_process_when_diagnostics_are_on() {
	let bin = super::built(&["--bin", "cartridge"]);
	let dir = tempfile::tempdir().unwrap();
	let home = dir.path().join("home");
	super::write(dir.path(), ".cartridge/init.lua", "return {}");
	super::write(dir.path(), ".cartridge/config.lua", "this is not lua ((");
	let clean = |mut child: std::process::Child| {
		let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
		while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
			std::thread::sleep(std::time::Duration::from_millis(50));
		}
		let exited = child.try_wait().unwrap().is_some();
		let _ = child.kill();
		exited
	};
	let trust = std::process::Command::new(&bin)
		.arg("trust")
		.arg(dir.path())
		.env("CARTRIDGE_HOME", &home)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
		.unwrap();
	assert!(clean(trust), "`cartridge trust` never returned");
	let child = std::process::Command::new(bin)
		.arg("socket")
		.current_dir(dir.path())
		.env("CARTRIDGE_HOME", &home)
		.env(
			"CARTRIDGE_DIAGNOSTICS",
			dir.path().join("diagnostics.jsonl"),
		)
		.env_remove("CARTRIDGE_DIAGNOSTICS_MAX_BYTES")
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
		.unwrap();
	assert!(clean(child), "`cartridge socket` never returned");
	let written = std::fs::read_to_string(dir.path().join("diagnostics.jsonl")).unwrap_or_default();
	assert!(
		written.contains("using declared defaults"),
		"the warning never reached the file: {written:?}"
	);
}
