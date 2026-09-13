use super::*;
#[test]
fn descriptors_invalidate_on_switch_and_recreation() {
	let service = Service::new(Arc::new(1u64), Reload::default());
	let before = service.version();
	assert_eq!(service.version(), before);
	service.switch(Arc::new(2u64));
	assert_ne!(service.version(), before);
	assert_eq!(service.value().downcast_ref::<u64>(), Some(&2));
	let recreated = Service::new(Arc::new(3u64), Reload::default());
	assert_ne!(recreated.version(), service.version());
}
