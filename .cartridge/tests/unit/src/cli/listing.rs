use super::*;
use cartridge::loader::Entry;

fn info(id: &str, events: &[&str], needs: &[&str], listen: &[&str]) -> CartridgeInfo {
	let strings = |names: &[&str]| names.iter().map(|n| n.to_string()).collect();
	CartridgeInfo {
		entry: Entry {
			id: id.into(),
			path: id.into(),
			config: serde_json::Value::Null,
			disabled: false,
			inject: Vec::new(),
		},
		needs: strings(needs),
		events: strings(events),
		listen: strings(listen),
		grant: None,
		unread: None,
		error: None,
	}
}

#[test]
fn the_listing_follows_sends_and_marks_cycles_and_missing_listeners() {
	let all = vec![
		info("left", &["ping"], &["missing"], &["pong"]),
		info("right", &["pong"], &[], &["ping"]),
	];
	let mut out = Vec::new();
	sends(&all, &all[0], 1, &mut vec!["left".into()], &mut out);
	assert_eq!(
		out,
		vec![
			"  ping -> right",
			"    pong -> left (cycle)",
			"  missing -> ?",
		]
	);
}
