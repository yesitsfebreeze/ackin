use super::*;

/// A stray key survives `merge`; `deny_unknown_fields` must not turn it into a
/// panic in every process. A declared pin still arrives.
#[test]
fn an_undeclared_host_key_leaves_the_declared_defaults_standing() {
	let mut stray = defaults(host_specs());
	crate::settings::merge(&mut stray, json!({"startup_timeout": 600}));
	assert_eq!(typed(stray).startup_timeout_secs, 60);
	let mut pinned = defaults(host_specs());
	crate::settings::merge(&mut pinned, json!({"startup_timeout_secs": 600}));
	assert_eq!(typed(pinned).startup_timeout_secs, 600);
}
