use super::*;
use crate::{reload::Reload, runtime::Runtime};
fn fixture() -> (Arc<Observer>, Arc<Mutex<Vec<Value>>>) {
	let rows = Arc::new(Mutex::new(Vec::new()));
	let output = rows.clone();
	let observer = Arc::new(Observer::new(
		r#"{"agent":{"actor":"agent","activity":"deliberate"},"ui":{"actor":"ui","activity":"read"},"poller":{"actor":"background","activity":"poll"}}"#,
		Arc::new(move |_, row| output.lock().unwrap().push(row)),
	));
	(observer, rows)
}
fn finish(observer: &Arc<Observer>, source: &str, op: &str, value: Result<Value, String>) -> Value {
	let mut guard = observer.begin(source, "tool.fixture", op);
	guard.state.lock().unwrap().value["dispatched"] = json!(true);
	guard.finish(Some(&value));
	let row = guard.state.lock().unwrap().value.clone();
	row
}
#[test]
fn attribution_is_host_configuration_and_discovery_is_distinct() {
	let (observer, rows) = fixture();
	for (source, op, actor, activity) in [
		("agent", "call", json!("agent"), json!("deliberate")),
		("ui", "call", json!("ui"), json!("read")),
		("poller", "call", json!("background"), json!("poll")),
		("other", "call", Value::Null, Value::Null),
		("agent", "describe", json!("agent"), json!("discovery")),
	] {
		let row = finish(
			&observer,
			source,
			op,
			Ok(json!({"content":"SECRET", "error":false,"actor":"forged"})),
		);
		assert_eq!(row["actor"], actor);
		assert_eq!(row["activity"], activity);
	}
	let rows = rows.lock().unwrap();
	assert_eq!(rows.len(), 10);
	assert!(!json!(*rows).to_string().contains("SECRET"));
	assert!(!json!(*rows).to_string().contains("forged"));
	assert!(rows[0]["elapsed_ms"].is_null());
	assert!(rows[0]["completion_known"].is_null());
}
#[test]
fn outcomes_never_treat_transport_or_cancellation_as_known_effect_completion() {
	let (observer, _) = fixture();
	for (op, result, outcome, known) in [
		(
			"call",
			Ok(json!({"content":"ok","error":false})),
			"success",
			true,
		),
		(
			"call",
			Ok(json!({"content":"SECRET","error":true})),
			"tool_error",
			true,
		),
		("call", Err("SECRET".into()), "transport_error", false),
		(
			"call",
			Ok(json!({"invalid":"SECRET"})),
			"invalid_response",
			false,
		),
		(
			"call",
			Ok(json!({"content":"interrupted-outcome-unknown","error":true})),
			"interrupted",
			false,
		),
		(
			"cancel",
			Ok(json!({"cancelled":true})),
			"cancel_acknowledged",
			false,
		),
	] {
		let row = finish(&observer, "agent", op, result);
		assert_eq!(row["outcome"], outcome);
		assert_eq!(row["completion_known"], known);
		assert!(!row.to_string().contains("SECRET"));
	}
}
#[test]
fn actual_descriptor_shapes_are_hashed_and_invalid_oversized_descriptors_are_unknown() {
	let first = json!({"name":"fixture","description":"one","input_schema":{"type":"object"}});
	let second = json!({"name":"fixture","description":"two","input_schema":{"type":"object"}});
	let a = descriptor(&first).unwrap();
	let b = descriptor(&second).unwrap();
	assert_ne!(a, b);
	assert_eq!(
		descriptor(&json!({"content":first.to_string(),"error":false})),
		Some(a.clone())
	);
	assert!(descriptor(&json!({"content":"not a descriptor","error":false})).is_none());
	assert!(
		descriptor(&json!({"name":"f","description":"s".repeat(70000),"input_schema":{}}))
			.is_none()
	);
	let (observer, _) = fixture();
	observer.described("generation1", "tool.fixture", a.clone());
	observer.described("generation1", "tool.fixture", b.clone());
	assert_eq!(observer.revision("generation1", "tool.fixture"), Some(b));
	assert_eq!(observer.revision("generation2", "tool.fixture"), None);
	for i in 0..128 {
		observer.described(&i.to_string(), "tool.fixture", a.clone());
	}
	assert_eq!(observer.cache.lock().unwrap().len(), 128);
	assert_eq!(observer.revision("generation1", "tool.fixture"), None);
}
#[tokio::test]
async fn intended_service_capture_ignores_nested_services_and_resumes_become_unknown() {
	let (observer, _) = fixture();
	let mut guard = observer.begin("agent", "tool.fixture", "call");
	let intended = Service::new(
		Arc::new(json!({"content":"ok","error":false})),
		Reload::default(),
	);
	let nested = Service::new(Arc::new(json!({})), Reload::default());
	guard.state.lock().unwrap().target = Some(&intended as *const Service as usize);
	let dir = tempfile::tempdir().unwrap();
	let host = Host::new(Runtime::new(), dir.path(), dir.path());
	ACTIVE
		.scope(
			Active {
				observer: observer.clone(),
				state: guard.state.clone(),
			},
			async {
				nested.call(&host, json!(null)).await.unwrap();
				assert!(guard.state.lock().unwrap().value["provider_generation"].is_null());
				intended.call(&host, json!(null)).await.unwrap();
				let generation = guard.state.lock().unwrap().value["provider_generation"].clone();
				assert!(generation
					.as_str()
					.unwrap()
					.ends_with(&format!(":{}", intended.version())));
				nested.call(&host, json!(null)).await.unwrap();
				assert_eq!(
					guard.state.lock().unwrap().value["provider_generation"],
					generation
				);
				resuming(&nested);
				assert_eq!(
					guard.state.lock().unwrap().value["provider_generation"],
					generation
				);
				resuming(&intended);
				assert!(guard.state.lock().unwrap().value["provider_generation"].is_null());
			},
		)
		.await;
	guard.finish(Some(&Ok(json!({"content":"ok","error":false}))));
	assert_eq!(guard.state.lock().unwrap().value["resumed"], true);
}
#[tokio::test]
async fn dropping_live_invocation_records_unknown_without_retry() {
	let (observer, rows) = fixture();
	let (ready, wait) = tokio::sync::oneshot::channel();
	let task = tokio::spawn(async move {
		let guard = observer.begin("agent", "tool.fixture", "call");
		guard.state.lock().unwrap().value["dispatched"] = json!(true);
		ready.send(()).unwrap();
		std::future::pending::<()>().await;
		drop(guard);
	});
	wait.await.unwrap();
	task.abort();
	assert!(task.await.unwrap_err().is_cancelled());
	let rows = rows.lock().unwrap();
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[1]["outcome"], "interrupted");
	assert_eq!(rows[1]["completion_known"], false);
}
#[test]
fn invalid_host_config_and_poisoned_cache_do_not_create_authority_or_fail_outcomes() {
	let observer = Arc::new(Observer::new(
		r#"{"agent":{"actor":"forged","activity":"deliberate"}}"#,
		Arc::new(|_, _| {}),
	));
	assert!(observer.actors.is_empty());
	let copy = observer.clone();
	let _ = std::thread::spawn(move || {
		let _lock = copy.cache.lock().unwrap();
		panic!("fixture poisoned cache");
	})
	.join();
	assert!(observer.revision("g", "tool.fixture").is_none());
	observer.described("g", "tool.fixture", "a".into());
	assert_eq!(
		finish(
			&observer,
			"agent",
			"call",
			Ok(json!({"content":"ok","error":false}))
		)["outcome"],
		"success"
	);
}
