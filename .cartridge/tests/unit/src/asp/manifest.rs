//! The `asp` block a manifest may declare.

use serde_json::{json, Value};

use crate::tests::write;

#[test]
fn a_manifest_refuses_an_asp_block_it_cannot_serve() {
	let dir = tempfile::tempdir().unwrap();
	let base = |asp: Value, listen: Value| json!({"name": "p", "entry": "init.lua", "events": {"asp.p": {}, "asp.q": {}}, "listen": listen, "asp": asp});
	let refused = [
		base(json!({"schemes": {"file": {}}}), json!([])),
		base(json!({"schemes": {"file": {}}}), json!(["asp.p", "asp.q"])),
		base(json!({"schemes": {"File:": {}}}), json!(["asp.p"])),
		base(json!({"attributes": {"other.size": {}}}), json!(["asp.p"])),
		base(json!({"colours": {}}), json!(["asp.p"])),
		json!({"name": "p", "entry": "init.lua", "events": {"asp": {}}}),
	];
	for manifest in refused {
		write(dir.path(), "cartridge.json", &manifest.to_string());
		assert!(
			crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).is_err(),
			"{manifest}"
		);
	}
	let accepted = base(
		json!({"schemes": {"file": {}}, "attributes": {"p.size": {}}}),
		json!(["asp.p"]),
	);
	write(dir.path(), "cartridge.json", &accepted.to_string());
	write(dir.path(), "init.lua", "");
	crate::loader::Cartridge::document(&dir.path().join("cartridge.json")).unwrap();
}
