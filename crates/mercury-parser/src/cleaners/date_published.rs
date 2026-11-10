//! Port of mercury-parser's `cleanDatePublished` (src/cleaners/date-published.js).
//!
//! The JS version leans on `new Date(string)` and dayjs; both are emulated
//! in the submodules so results match the JS implementation run with
//! `TZ=UTC` (offset-less strings are read as UTC).

mod dayjs;
mod js_date;

use std::sync::LazyLock;

use regex::Regex;

/// Contents of a character class matching JS `\s`.
const JS_WS: &str = r"\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";
/// JS `.` (anything but a line terminator).
const JS_DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

static MS_DATE_STRING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{13}$").unwrap());
static SEC_DATE_STRING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{10}$").unwrap());
static TIME_WITH_OFFSET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([+-][0-9]{2}:?[0-9]{2}|Z)$").unwrap());
// JS: /(?<!\d)(\d+)\s+(seconds?|...|years?)\s+ago/i. The lookbehind is
// emulated with a leading `^|[^0-9]`.
static TIME_AGO_STRING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:^|[^0-9])([0-9]+)[{JS_WS}]+((?i-u:seconds?|minutes?|hours?|days?|weeks?|months?|years?))[{JS_WS}]+(?i-u:ago)"
    ))
    .unwrap()
});
static TIME_NOW_STRING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[{JS_WS}]*(?i-u:just|right)?[{JS_WS}]*(?i-u:now)"
    ))
    .unwrap()
});
static HAS_TIMEZONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[+-][0-9]{2}:?[0-9]{2}|(?i-u:z)|(?-u:\b)(?i-u:gmt)[+-][0-9]+|(?-u:\b)(?i-u:utc)(?-u:\b)",
    )
    .unwrap()
});
static TIMEZONE_ABBR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?-u:\b)(?i-u:EST|EDT|CST|CDT|MST|MDT|PST|PDT|ET|CT|MT|PT|GMT|UTC)(?-u:\b)")
        .unwrap()
});
static WHITESPACE_RUN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("[{JS_WS}]+")).unwrap());
static FORMAT_TIMEZONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("[{JS_WS}]*(?i-u:z)+")).unwrap());
static SPLIT_DATE_STRING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"([0-9]{{1,2}}:[0-9]{{2}}( ?(?i-u:[ap]){JS_DOT}?(?i-u:m){JS_DOT}?)?)|([0-9]{{1,2}}[/-][0-9]{{1,2}}[/-][0-9]{{2,4}})|(-[0-9]{{3,4}}$)|([0-9]{{1,4}})|((?i-u:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec))"
    ))
    .unwrap()
});
static TIME_MERIDIAN_DOTS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i-u:\.m\.)").unwrap());
static TIME_MERIDIAN_SPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({JS_DOT}*[0-9])((?i-u:am|pm))({JS_DOT}*)")).unwrap());
static CLEAN_DATE_STRING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[{JS_WS}]*(?i-u:published)[{JS_WS}]*:?[{JS_WS}]*({JS_DOT}*)"
    ))
    .unwrap()
});

/// Mirrors cleanDatePublished(dateString, { timezone, format }).
/// Returns an ISO-8601 string exactly like JS `Date.prototype.toISOString()`,
/// e.g. "2016-11-22T13:57:00.000Z", or None when invalid.
pub fn clean_date_published(
    date_string: &str,
    timezone: Option<&str>,
    format: Option<&str>,
) -> Option<String> {
    let now = chrono::Utc::now().timestamp_millis() as f64;
    clean_date_published_at(date_string, timezone, format, now)
}

/// Mirrors cleanDateString: keeps only the date-ish fragments of a string.
pub fn clean_date_string(date_string: &str) -> String {
    let joined = SPLIT_DATE_STRING
        .find_iter(date_string)
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let s = TIME_MERIDIAN_DOTS_RE.replace(&joined, "m");
    let s = TIME_MERIDIAN_SPACE_RE.replace(&s, "${1} ${2} ${3}");
    let s = CLEAN_DATE_STRING_RE.replace(&s, "${1}");
    js_trim(&s).to_string()
}

/// `clean_date_published` relative to `now` (JS time value, ms since epoch).
fn clean_date_published_at(
    date_string: &str,
    timezone: Option<&str>,
    format: Option<&str>,
    now: f64,
) -> Option<String> {
    if MS_DATE_STRING.is_match(date_string) {
        return js_date::to_iso_string(js_date::time_clip(date_string.parse().ok()?));
    }
    if SEC_DATE_STRING.is_match(date_string) {
        let secs: f64 = date_string.parse().ok()?;
        return js_date::to_iso_string(js_date::time_clip(secs * 1000.0));
    }
    // JS truthiness: empty options count as absent.
    let timezone = timezone.filter(|tz| !tz.is_empty());
    let format = format.filter(|f| !f.is_empty());

    let mut date = create_date(date_string, timezone, format, now)?;
    if date.is_nan() {
        date = create_date(&clean_date_string(date_string), timezone, format, now)?;
    }
    js_date::to_iso_string(date)
}

// Mirrors createDate. Returns the instant (NaN when invalid), or `None`
// where the JS implementation throws (an unknown timezone).
fn create_date(
    date_string: &str,
    timezone: Option<&str>,
    format: Option<&str>,
    now: f64,
) -> Option<f64> {
    if TIME_WITH_OFFSET_RE.is_match(date_string) {
        return Some(js_date::parse(date_string));
    }
    if let Some(caps) = TIME_AGO_STRING.captures(date_string) {
        return Some(dayjs::subtract(now, &caps[1], &caps[2]));
    }
    if TIME_NOW_STRING.is_match(date_string) {
        return Some(now);
    }

    let has_timezone = HAS_TIMEZONE_RE.is_match(date_string);
    let cleaned = strip_timezone_abbr(date_string);

    if has_timezone {
        let native = js_date::parse(date_string);
        if !native.is_nan() {
            return Some(native);
        }
    }

    if let Some(timezone) = timezone
        && !has_timezone
    {
        if let Some(format) = format {
            let parsed =
                dayjs::tz_parse(&cleaned, &strip_timezone_from_format(format), timezone, now);
            if !parsed.is_nan() {
                return Some(parsed);
            }
        }
        let native = js_date::parse(&cleaned);
        if !native.is_nan() {
            return dayjs::tz_keep_local_time(native, timezone);
        }
        let parsed = dayjs::parse(&cleaned);
        if !parsed.is_nan() {
            return dayjs::tz_keep_local_time(parsed, timezone);
        }
        return Some(f64::NAN);
    }

    if let Some(format) = format {
        let parsed =
            dayjs::parse_with_format(&cleaned, &strip_timezone_from_format(format), false, now);
        if !parsed.is_nan() {
            return Some(parsed);
        }
    }
    let native = js_date::parse(&cleaned);
    if !native.is_nan() {
        return Some(native);
    }
    Some(dayjs::parse(&cleaned))
}

fn strip_timezone_abbr(s: &str) -> String {
    let s = TIMEZONE_ABBR_RE.replace_all(s, "");
    let s = WHITESPACE_RUN.replace_all(&s, " ");
    js_trim(&s).to_string()
}

// Removes timezone tokens (zz, z, ZZ, Z) from a format string.
fn strip_timezone_from_format(format: &str) -> String {
    let s = FORMAT_TIMEZONE_RE.replace_all(format, "");
    let s = WHITESPACE_RUN.replace_all(&s, " ");
    js_trim(&s).to_string()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| js_date::is_js_whitespace(c as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2020-06-15T12:34:56.789Z
    const NOW: f64 = 1_592_224_496_789.0;

    fn at_now(s: &str) -> Option<String> {
        clean_date_published_at(s, None, None, NOW)
    }

    #[test]
    fn relative_dates() {
        let cases = [
            ("5 hours ago", "2020-06-15T07:34:56.789Z"),
            ("30 seconds ago", "2020-06-15T12:34:26.789Z"),
            ("1 minute ago", "2020-06-15T12:33:56.789Z"),
            ("3 days ago", "2020-06-12T12:34:56.789Z"),
            ("2 weeks ago", "2020-06-01T12:34:56.789Z"),
            ("4 months ago", "2020-02-15T12:34:56.789Z"),
            ("6 years ago", "2014-06-15T12:34:56.789Z"),
            ("Posted 2 Days Ago", "2020-06-13T12:34:56.789Z"),
            ("now", "2020-06-15T12:34:56.789Z"),
            ("just now", "2020-06-15T12:34:56.789Z"),
            ("Right Now!", "2020-06-15T12:34:56.789Z"),
        ];
        for (input, expected) in cases {
            assert_eq!(at_now(input).as_deref(), Some(expected), "{input:?}");
        }
    }

    #[test]
    fn month_subtraction_clamps_day() {
        // 2020-03-31T00:00:00.000Z
        let now = 1_585_612_800_000.0;
        assert_eq!(
            clean_date_published_at("1 month ago", None, None, now).as_deref(),
            Some("2020-02-29T00:00:00.000Z")
        );
    }

    /// Every case in tests/date_corpus.json (generated from the JS
    /// implementation by js/build_corpus.js) must match exactly, evaluated
    /// against the frozen "now" it was generated with.
    #[test]
    fn matches_js_corpus() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/dates/date_corpus.json");
        let corpus: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut failures = Vec::new();
        for case in &corpus {
            let input = case["input"].as_str().unwrap();
            let timezone = case["timezone"].as_str();
            let format = case["format"].as_str();
            let now = case["now"].as_f64().unwrap();
            let expected = case["expected"].as_str();
            let actual = clean_date_published_at(input, timezone, format, now);
            if actual.as_deref() != expected {
                failures.push(format!(
                    "{input:?} tz={timezone:?} fmt={format:?} now={now}: expected {expected:?}, got {actual:?}"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} of {} corpus cases failed:\n{}",
            failures.len(),
            corpus.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn clean_date_string_extracts_fragments() {
        assert_eq!(
            clean_date_string("Published: Dec 12, 2016 5:00pm"),
            "Dec 12 2016 5:00 pm"
        );
        assert_eq!(clean_date_string("5:00 a.m. PT"), "5:00 am");
        assert_eq!(clean_date_string("nothing here"), "");
    }
}
