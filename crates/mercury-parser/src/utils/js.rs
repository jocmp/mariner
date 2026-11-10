//! Helpers that reproduce JavaScript string semantics.
//!
//! Mercury's heuristics lean on `String.prototype.length` (UTF-16 code
//! units), `trim()`, `\s`, `parseInt`, and `parseFloat`. Rust's equivalents
//! differ at the edges (e.g. `char::is_whitespace` includes U+0085 but not
//! U+FEFF), and those edges show up in real pages, so parity code goes
//! through these instead.

/// The characters JavaScript's `\s` and `trim()` treat as whitespace, as a
/// regex character-class body.
#[macro_export]
#[doc(hidden)]
macro_rules! js_ws {
    () => {
        r"\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}"
    };
}

/// Whether `c` is whitespace according to JavaScript (`WhiteSpace` or
/// `LineTerminator`).
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim()`.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_whitespace)
}

/// `String.prototype.length`: the number of UTF-16 code units.
pub fn len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// `String.prototype.slice(0, end)` with `end` in UTF-16 code units. A
/// surrogate pair split by `end` is dropped rather than halved.
pub fn slice_to(s: &str, end: usize) -> &str {
    let mut units = 0;
    for (index, c) in s.char_indices() {
        units += c.len_utf16();
        if units > end {
            return &s[..index];
        }
    }
    s
}

/// `parseFloat(s)`, returning `None` where JS returns `NaN`.
pub fn parse_float(s: &str) -> Option<f64> {
    let s = s.trim_start_matches(is_whitespace);
    let bytes = s.as_bytes();
    let mut end = 0;

    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end += 1;
    }
    if s[end..].starts_with("Infinity") {
        let infinity = if bytes.first() == Some(&b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        return Some(infinity);
    }

    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut mantissa_digits = end - digits_start;
    if end < bytes.len() && bytes[end] == b'.' {
        let fraction_start = end + 1;
        let mut fraction_end = fraction_start;
        while fraction_end < bytes.len() && bytes[fraction_end].is_ascii_digit() {
            fraction_end += 1;
        }
        mantissa_digits += fraction_end - fraction_start;
        if mantissa_digits > 0 {
            end = fraction_end;
        }
    }
    if mantissa_digits == 0 {
        return None;
    }

    if end < bytes.len() && matches!(bytes[end], b'e' | b'E') {
        let mut exponent_end = end + 1;
        if matches!(bytes.get(exponent_end), Some(b'+' | b'-')) {
            exponent_end += 1;
        }
        let exponent_digits_start = exponent_end;
        while exponent_end < bytes.len() && bytes[exponent_end].is_ascii_digit() {
            exponent_end += 1;
        }
        if exponent_end > exponent_digits_start {
            end = exponent_end;
        }
    }

    s[..end].parse().ok()
}

/// `parseInt(s, 10)`, returning `None` where JS returns `NaN`.
pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim_start_matches(is_whitespace);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let end = digits
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return None;
    }

    // Saturate instead of failing: JS would produce a large float here.
    let value = digits[..end].parse::<i64>().unwrap_or(i64::MAX);
    Some(if negative { -value } else { value })
}

/// Format a number the way JS's `String(number)` does for the values
/// Mercury writes into `score` attributes.
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if n == n.trunc() && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }

    // Rust's shortest round-trip formatting matches JS for the magnitudes
    // that come up in scoring.
    format!("{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_js_whitespace() {
        assert_eq!(trim("\u{feff} hi\u{a0}\n"), "hi");
        assert_eq!(trim("\u{85}hi"), "\u{85}hi");
    }

    #[test]
    fn counts_utf16_units() {
        assert_eq!(len("abc"), 3);
        assert_eq!(len("日本"), 2);
        assert_eq!(len("😀"), 2);
    }

    #[test]
    fn slices_by_utf16_units() {
        assert_eq!(slice_to("日本語", 2), "日本");
        assert_eq!(slice_to("a😀b", 2), "a");
        assert_eq!(slice_to("abc", 10), "abc");
    }

    #[test]
    fn parses_floats_like_js() {
        assert_eq!(parse_float("12.5px"), Some(12.5));
        assert_eq!(parse_float("  -3"), Some(-3.0));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("5."), Some(5.0));
        assert_eq!(parse_float("1e3x"), Some(1000.0));
        assert_eq!(parse_float("abc"), None);
        assert_eq!(parse_float(""), None);
    }

    #[test]
    fn parses_ints_like_js() {
        assert_eq!(parse_int("42px"), Some(42));
        assert_eq!(parse_int("-7"), Some(-7));
        assert_eq!(parse_int("3.9"), Some(3));
        assert_eq!(parse_int("px"), None);
    }

    #[test]
    fn formats_numbers_like_js() {
        assert_eq!(number_to_string(5.0), "5");
        assert_eq!(number_to_string(-2.5), "-2.5");
        assert_eq!(number_to_string(0.1 + 0.2), "0.30000000000000004");
    }
}
