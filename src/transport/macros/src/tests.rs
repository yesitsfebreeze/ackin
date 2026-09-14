use super::to_snake;

#[test]
fn to_snake_lowercases_and_word_splits_on_interior_capitals() {
	assert_eq!(to_snake("MemoryRpc"), "memory_rpc");
	assert_eq!(to_snake("MemoryRpc"), "memory_rpc");
	assert_eq!(to_snake("SearchSvc"), "search_svc");
	assert_eq!(to_snake("Memory"), "memory");
	assert_eq!(to_snake("X"), "x");
	assert_eq!(to_snake("ABC"), "a_b_c");
	assert_eq!(to_snake("already_snake"), "already_snake");
	assert_eq!(to_snake(""), "");
}
