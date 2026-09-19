use super::*;
use serde_json::json;

fn manifest(event: serde_json::Value) -> String {
	json!({"name": "a", "entry": "init.lua", "events": {"a.said": event}}).to_string()
}

fn parsed(event: serde_json::Value) -> Result<Cartridge> {
	Cartridge::parse(Path::new("a/cartridge.json"), &manifest(event))
}

#[test]
fn each_frame_in_the_closed_set_loads() {
	for (name, frame) in [
		("message", Frame::Message),
		("data", Frame::Data),
		("none", Frame::None),
	] {
		let cartridge = parsed(json!({"frame": name})).unwrap();
		assert_eq!(cartridge.events["a.said"].frame, Some(frame), "{name}");
	}
}

#[test]
fn a_frame_outside_the_closed_set_is_refused_naming_its_kind_and_value() {
	for value in ["system", "developer", "Message"] {
		let error = parsed(json!({"frame": value}))
			.err()
			.unwrap_or_else(|| panic!("`{value}` loaded"))
			.to_string();
		assert!(error.contains("events.a.said"), "{error}");
		assert!(error.contains(&format!("`{value}`")), "{error}");
		assert!(error.contains("a/cartridge.json"), "{error}");
	}
}

#[test]
fn a_declaration_without_a_frame_round_trips_unchanged() {
	let declared = json!({"description": "said", "schema": {"type": "object"}, "timeout_ms": 5});
	let cartridge = parsed(declared.clone()).unwrap();
	assert_eq!(cartridge.events["a.said"].frame, None);
	assert_eq!(json!(cartridge.events["a.said"]), declared);
}

#[test]
fn every_current_manifest_loads_its_events_unchanged() {
	let fixtures =
		Path::new(env!("CARGO_MANIFEST_DIR")).join(".cartridge/tests/unit/fixtures/manifests");
	let mut loaded = 0;
	for entry in std::fs::read_dir(&fixtures).unwrap() {
		let path = entry.unwrap().path();
		let source = std::fs::read_to_string(&path).unwrap();
		let cartridge = Cartridge::parse(&path, &source).unwrap_or_else(|e| panic!("{e}"));
		let raw: serde_json::Value = serde_json::from_str(&source).unwrap();
		let declared = raw.get("events").cloned().unwrap_or_else(|| json!({}));
		let plain: std::collections::BTreeMap<String, Event> =
			serde_json::from_value(declared.clone()).unwrap();
		assert_eq!(json!(cartridge.events), declared, "{}", path.display());
		assert_eq!(json!(cartridge.events), json!(plain), "{}", path.display());
		assert!(cartridge.events.values().all(|event| event.frame.is_none()));
		loaded += cartridge.events.len();
	}
	assert!(loaded > 0, "no fixture declares an event");
}
