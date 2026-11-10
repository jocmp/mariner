//! Truncate text at a word boundary, from `src/utils/ellipsize/`.

/// Truncate `s` to at most `max` UTF-16 code units, breaking on the last
/// space or hyphen that fits and appending `ellipse`.
pub fn ellipsize(s: &str, max: usize, ellipse: &str) -> String {
    // Work in UTF-16 code units, like the JS `charAt`/`slice` calls this
    // is ported from.
    let units: Vec<u16> = s.encode_utf16().collect();
    if units.is_empty() || max == 0 {
        return String::new();
    }
    if units.len() <= max {
        return s.to_string();
    }

    let max_len = max.saturating_sub(ellipse.encode_utf16().count());
    let end = (0..=max_len)
        .rev()
        .find(|&i| matches!(units.get(i), Some(&c) if c == u16::from(b' ') || c == u16::from(b'-')))
        .unwrap_or(max_len);

    format!(
        "{}{ellipse}",
        String::from_utf16_lossy(&units[..end.min(units.len())])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_short_strings_alone() {
        assert_eq!(ellipsize("short", 10, "…"), "short");
    }

    #[test]
    fn breaks_on_word_boundaries() {
        assert_eq!(ellipsize("one two three four", 12, "…"), "one two…");
        assert_eq!(
            ellipsize("one two three four", 12, "&hellip;"),
            "one&hellip;"
        );
    }

    #[test]
    fn truncates_without_breakpoints() {
        assert_eq!(ellipsize("abcdefghij", 5, "…"), "abcd…");
    }
}
