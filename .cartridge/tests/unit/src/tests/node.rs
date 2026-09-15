#[test]
fn the_entry_recheck_refuses_bytes_that_changed() {
	let dir = tempfile::tempdir().unwrap();
	let entry = dir.path().join("init.lua");
	std::fs::write(&entry, "return {}").unwrap();
	let digest = crate::trust::digest(&entry).unwrap();
	crate::node::entry_bytes(&entry, &digest).unwrap();
	std::fs::write(&entry, "return { {} }").unwrap();
	let refused = crate::node::entry_bytes(&entry, &digest).unwrap_err();
	assert!(
		refused
			.to_string()
			.contains("changed since the base verified it"),
		"{refused}"
	);
}
