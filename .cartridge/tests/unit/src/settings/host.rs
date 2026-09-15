use super::*;

// A stray key survives `merge`: `deny_unknown_fields` must not turn it into a
// panic in every process.
#[test]
fn an_undeclared_host_key_leaves_the_declared_defaults_standing() {
	let mut stray = defaults(host_specs());
	crate::settings::merge(&mut stray, json!({"startup_timeout": 600}));
	let mut refused = Vec::new();
	assert_eq!(typed(stray, &mut refused).startup_timeout_secs, 60);
	assert_eq!(refused.len(), 1, "{refused:?}");
	let mut pinned = defaults(host_specs());
	crate::settings::merge(&mut pinned, json!({"startup_timeout_secs": 600}));
	let mut kept = Vec::new();
	assert_eq!(typed(pinned, &mut kept).startup_timeout_secs, 600);
	assert!(kept.is_empty(), "{kept:?}");
}
