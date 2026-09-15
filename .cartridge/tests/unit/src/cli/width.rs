use super::*;

#[test]
fn fit_and_pad_count_display_cells() {
	let cjk = "設定設定設定設定設定設定";
	assert_eq!(cells(cjk), 24);
	assert_eq!(fit(cjk, 20), "設定設定設定設定設定");
	assert_eq!(fit(cjk, 1), "");
	let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
	assert_eq!(cells(family), 2);
	assert_eq!(fit(family, 1), "");
	assert_eq!(pad("設定", 6), "設定  ");
	assert_eq!(pad_start("設定", 6), "  設定");
	assert_eq!(fit("a\tb", 10), "a    b");
}
