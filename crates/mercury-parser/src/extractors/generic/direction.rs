//! Text direction, via the `string-direction` package upstream.

use std::sync::LazyLock;

use regex::Regex;

use crate::js_ws;

static STRIP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), r#"\x00'"\-0-9+?!]+"#)).unwrap());

const LTR_MARK: char = '\u{200e}';
const RTL_MARK: char = '\u{200f}';

// Hebrew, Arabic, NKo, Syriac, Thaana, and Tifinagh. The bounds are
// exclusive, as upstream.
const RTL_SCRIPT_RANGES: &[(u16, u16)] = &[
    (0x0590, 0x05FF),
    (0x0600, 0x06FF),
    (0x07C0, 0x07FF),
    (0x0700, 0x074F),
    (0x0780, 0x07BF),
    (0x2D30, 0x2D7F),
];

/// `"ltr"`, `"rtl"`, `"bidi"`, or `""` when the text has no direction.
pub fn direction(text: &str) -> &'static str {
    if text.is_empty() {
        return "";
    }

    let has_ltr_mark = text.contains(LTR_MARK);
    let has_rtl_mark = text.contains(RTL_MARK);
    match (has_ltr_mark, has_rtl_mark) {
        (true, true) => return "bidi",
        (true, false) => return "ltr",
        (false, true) => return "rtl",
        _ => {}
    }

    let has_digit = text.bytes().any(|b| b.is_ascii_digit());
    let stripped = STRIP_RE.replace_all(text, "");
    let mut has_rtl = false;
    let mut has_ltr = false;
    for unit in stripped.encode_utf16() {
        if RTL_SCRIPT_RANGES
            .iter()
            .any(|&(from, to)| unit > from && unit < to)
        {
            has_rtl = true;
        } else {
            has_ltr = true;
        }
    }
    let has_ltr = has_ltr || (!has_rtl && has_digit);

    match (has_ltr, has_rtl) {
        (true, true) => "bidi",
        (true, false) => "ltr",
        (false, true) => "rtl",
        (false, false) => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direction() {
        assert_eq!(direction("Hello"), "ltr");
        assert_eq!(direction("שלום"), "rtl");
        assert_eq!(direction("Hello שלום"), "bidi");
        assert_eq!(direction("2024"), "ltr");
        assert_eq!(direction(""), "");
    }
}
