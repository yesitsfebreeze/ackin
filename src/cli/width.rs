use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) fn cells(text: &str) -> usize {
	text.graphemes(true).map(UnicodeWidthStr::width).sum()
}

pub(crate) fn fit(text: &str, width: usize) -> String {
	let spread = text.replace('\t', "    ");
	let mut out = String::new();
	let mut used = 0;
	for cluster in spread.graphemes(true) {
		let w = UnicodeWidthStr::width(cluster);
		if used + w > width {
			break;
		}
		out.push_str(cluster);
		used += w;
	}
	out
}

pub(crate) fn pad(text: &str, width: usize) -> String {
	format!("{text}{}", " ".repeat(width.saturating_sub(cells(text))))
}

pub(crate) fn pad_start(text: &str, width: usize) -> String {
	format!("{}{text}", " ".repeat(width.saturating_sub(cells(text))))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/cli/width.rs"]
mod tests;
