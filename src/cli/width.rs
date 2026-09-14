//! Terminal arithmetic for the listings: a glyph is a display cell, not a byte
//! and not a `char`. The unit is a grapheme cluster — summing per-`char`
//! widths measures a ZWJ family emoji as nine cells where a terminal draws two.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Display cells `text` occupies.
pub(crate) fn cells(text: &str) -> usize {
	text.graphemes(true).map(UnicodeWidthStr::width).sum()
}

/// `text` cut to `width` cells, tabs spread. A cluster that would straddle the
/// edge is dropped whole, so the answer never exceeds `width`.
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

/// `text` padded after to `width` cells; `{:width$}` pads by `char`.
pub(crate) fn pad(text: &str, width: usize) -> String {
	format!("{text}{}", " ".repeat(width.saturating_sub(cells(text))))
}

/// `text` padded before to `width` cells.
pub(crate) fn pad_start(text: &str, width: usize) -> String {
	format!("{}{text}", " ".repeat(width.saturating_sub(cells(text))))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/cli/width.rs"]
mod tests;
