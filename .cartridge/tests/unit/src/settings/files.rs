use super::*;

/// The answer comes from what `Sources::read` read, not off the disk again:
/// deleting the file after the read tells the two apart.
#[test]
fn the_listing_reads_each_configuration_file_once() {
	crate::tests::home();
	let dir = tempfile::tempdir().unwrap();
	let descriptor = dir.path().join(".cartridge");
	std::fs::create_dir_all(&descriptor).unwrap();
	let file = project_path(&descriptor);
	std::fs::write(&file, "return { host = { startup_timeout_secs = 600 } }").unwrap();
	// The read goes through the trust gate (`read` → `trust::read`).
	crate::trust::record(dir.path()).unwrap();
	let sources = Sources::read(&descriptor);
	std::fs::remove_file(&file).unwrap();
	assert_eq!(
		sources.of("host.startup_timeout_secs", &json!(600), &json!(60)),
		"project"
	);
	// A key no real ~/.cartridge/config.lua pins, so the developer's file cannot answer.
	assert_eq!(
		sources.of("nobody.pins_this", &json!(8), &json!(4096)),
		"descriptor"
	);
	assert_eq!(
		sources.of("nobody.pins_this", &json!(4096), &json!(4096)),
		"default"
	);
}
