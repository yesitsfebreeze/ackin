use super::*;

#[test]
fn a_credential_or_a_prompt_body_is_omitted_and_named() {
	let mut fields = serde_json::json!({
		"model": "m",
		"api_key": "sk-live-1",
		"request": { "messages": [{ "role": "user" }] },
	});
	let mut omitted = Vec::new();
	redact(&mut fields, &mut omitted);
	assert_eq!(fields["api_key"], "<omitted>");
	assert_eq!(fields["request"]["messages"], "<omitted>");
	assert_eq!(fields["model"], "m");
	omitted.sort();
	assert_eq!(omitted, ["api_key", "messages"]);
}

#[test]
fn the_sink_stays_bounded_by_rotating_one_generation() {
	let dir = std::env::temp_dir().join(format!("cartridge-diag-{}", mint()));
	std::fs::create_dir_all(&dir).unwrap();
	let path = dir.join("cartridge.jsonl");
	let mut sink = Sink::file(path.clone(), 64).unwrap();
	for i in 0..50 {
		sink.write(&format!("{{\"n\":{i},\"pad\":\"xxxxxxxxxxxxxxxx\"}}\n"));
	}
	let live = std::fs::metadata(&path).unwrap().len();
	assert!(live <= 64, "live generation is {live} bytes, cap is 64");
	assert!(
		dir.join("cartridge.jsonl.1").exists(),
		"no rotated generation"
	);
	let reopened = Sink::file(path.clone(), 64).unwrap();
	assert_eq!(reopened.written, live);
	std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn redaction_reaches_objects_inside_nested_arrays() {
	let mut fields =
		serde_json::json!({"events":[[{"api_key":"synthetic-token", "safe":"kept"}]], "count":1});
	let mut omitted = Vec::new();
	redact(&mut fields, &mut omitted);
	assert_eq!(fields["events"][0][0]["api_key"], "<omitted>");
	assert_eq!(fields["events"][0][0]["safe"], "kept");
	assert_eq!(omitted, ["api_key"]);
	assert!(!fields.to_string().contains("synthetic-token"));
}

#[test]
fn oversized_records_are_valid_json_and_bounded_before_and_after_rotation() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("diagnostic.jsonl");
	let mut sink = Sink::file(path.clone(), 64).unwrap();
	let huge = format!("{}\n", serde_json::json!({"text":"é🙂".repeat(100)}));
	sink.write(&huge);
	sink.write("{\"ordinary\":true}\n");
	sink.write(&huge);
	for file in [path, dir.path().join("diagnostic.jsonl.1")] {
		let data = std::fs::read_to_string(file).unwrap();
		assert!(data.len() <= 64);
		for line in data.lines() {
			let value: Json = serde_json::from_str(line).unwrap();
			assert!(value.get("omitted").is_some() || value["ordinary"] == true);
		}
	}
}

#[test]
fn a_sink_that_cannot_repair_a_failed_write_stops_accepting_records() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("diagnostic.jsonl");
	std::fs::write(&path, "{\"ordinary\":true}\n").unwrap();
	let mut sink = Sink::file(path.clone(), 64).unwrap();
	sink.out = Out::File(path.clone(), std::fs::File::open(&path).unwrap());
	sink.write("{\"next\":1}\n");
	assert!(matches!(sink.out, Out::Disabled));
	sink.write("{\"next\":2}\n");
	assert_eq!(
		std::fs::read_to_string(path).unwrap(),
		"{\"ordinary\":true}\n"
	);
}

#[test]
fn a_full_diagnostics_queue_drops_records_and_says_how_many() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("diagnostic.jsonl");
	let mut sink = Sink::file(path.clone(), 64 * 1024).unwrap();
	let (lines, notes) = std::sync::mpsc::sync_channel::<Note>(2);
	let dropped = AtomicU64::new(0);
	for i in 0..5 {
		if lines
			.try_send(Note::Line(format!("{{\"n\":{i}}}\n")))
			.is_err()
		{
			dropped.fetch_add(1, Ordering::Relaxed);
		}
	}
	drop(lines);
	drain(&mut sink, &notes, &dropped);
	let rows: Vec<Json> = std::fs::read_to_string(&path)
		.unwrap()
		.lines()
		.map(|line| serde_json::from_str(line).unwrap())
		.collect();
	assert_eq!(rows.len(), 3, "{rows:?}");
	assert_eq!(rows[0]["msg"], "diagnostics dropped");
	assert_eq!(rows[0]["dropped"], 3);
	assert_eq!(rows[1]["n"], 0);
	assert_eq!(rows[2]["n"], 1);
	assert_eq!(dropped.load(Ordering::Relaxed), 0);
}
