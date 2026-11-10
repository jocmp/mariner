//! Lenient date parsing for the many date formats feeds use.
//!
//! Finds a time of day (with optional AM/PM and zone) and a calendar date
//! anywhere in the string: ISO 8601 (`2024-10-05`), `yyyymmddHHMMSS`,
//! day-first numeric (`05.10.2024`, `05/10/2024`), or a month name with a
//! day and year in either order (`5 Oct 2024`, `October 5, 2024`,
//! `2024 Oct 05`). Weekday names are ignored. A missing time is midnight,
//! and a missing or unknown zone is UTC. Invalid dates give `None`.

use std::sync::LazyLock;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use regex::Regex;

pub(crate) fn parse(input: &str) -> Option<DateTime<Utc>> {
    static COMPACT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})\s*$").unwrap()
    });
    if let Some(c) = COMPACT.captures(input) {
        let n = |i: usize| c[i].parse::<u32>().ok();
        let date = NaiveDate::from_ymd_opt(c[1].parse().ok()?, n(2)?, n(3)?)?;
        let time = NaiveTime::from_hms_opt(n(4)?, n(5)?, n(6)?)?;
        return Some(NaiveDateTime::new(date, time).and_utc());
    }

    let (time, offset, rest) = match find_time(input) {
        Some((time, offset, rest)) => (time?, offset, rest),
        None => (NaiveTime::MIN, 0, input.to_string()),
    };
    let date = find_date(&rest)?;
    let local = NaiveDateTime::new(date, time);
    Some(local.and_utc() - Duration::seconds(offset))
}

/// Finds `hh:mm[:ss[.fff]] [am|pm] [zone]` (or `3pm`), returning the time
/// (`None` when out of range), the zone's offset in seconds and the input
/// with the match removed.
fn find_time(input: &str) -> Option<(Option<NaiveTime>, i64, String)> {
    static TIME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            (?:^|[^\d])
            (?P<all>
              (?P<h>\d{1,2})
              (?: :(?P<m>\d{2}) (?: :(?P<s>\d{2}) (?:[.,]\d+)? )? \s*(?P<ap>[ap])\.?m\b\.?
                | :(?P<m2>\d{2}) (?: :(?P<s2>\d{2}) (?:[.,]\d+)? )?
                | \s*(?P<ap2>[ap])\.?m\b\.?
              )
              (?P<zone>
                \s*(?:z\b|(?:gmt|utc?)?[-+]\d{1,2}(?::?\d{2})?|[a-z]{1,5}\b)
              )?
            )",
        )
        .unwrap()
    });
    let captures = TIME.captures(input)?;
    let time = time_of(&captures);
    let all = captures.name("all").expect("always matches");
    // A word after the time that's a month or weekday belongs to the date
    // (`10:00 Oct 5`), so it stays in the rest.
    let (offset, end) = match captures.name("zone") {
        Some(zone) => match offset_of(zone.as_str().trim()) {
            Some(offset) => (offset, all.end()),
            None => (0, zone.start()),
        },
        None => (0, all.end()),
    };
    let rest = format!("{} {}", &input[..all.start()], &input[end..]);
    Some((time, offset, rest))
}

fn time_of(captures: &regex::Captures) -> Option<NaiveTime> {
    let mut hour: u32 = captures["h"].parse().ok()?;
    let number = |a: &str, b: &str| {
        captures
            .name(a)
            .or(captures.name(b))
            .map_or(Some(0), |m| m.as_str().parse::<u32>().ok())
    };
    let (minute, second) = (number("m", "m2")?, number("s", "s2")?);
    if let Some(meridian) = captures.name("ap").or(captures.name("ap2")) {
        if hour == 0 || hour > 12 {
            return None;
        }
        hour %= 12;
        if meridian.as_str().eq_ignore_ascii_case("p") {
            hour += 12;
        }
    }
    NaiveTime::from_hms_opt(hour, minute, second)
}

/// The UTC offset, in seconds, of a zone written as `Z`, `+02:00`,
/// `-0500`, `GMT+2` or an abbreviation. Unknown abbreviations are UTC.
fn offset_of(zone: &str) -> Option<i64> {
    let lower = zone.to_ascii_lowercase();
    let numeric = lower
        .strip_prefix("gmt")
        .or_else(|| lower.strip_prefix("utc"))
        .or_else(|| lower.strip_prefix("ut"))
        .unwrap_or(&lower);
    if let Some(sign) = numeric.chars().next().filter(|c| *c == '+' || *c == '-') {
        let digits: String = numeric[1..].chars().filter(char::is_ascii_digit).collect();
        let (hours, minutes) = match digits.len() {
            1 | 2 => (digits.parse::<i64>().ok()?, 0),
            3 | 4 => {
                let split = digits.len() - 2;
                (
                    digits[..split].parse::<i64>().ok()?,
                    digits[split..].parse::<i64>().ok()?,
                )
            }
            _ => return None,
        };
        let seconds = hours * 3600 + minutes * 60;
        return Some(if sign == '-' { -seconds } else { seconds });
    }
    let hours = match lower.as_str() {
        "z" | "ut" | "utc" | "gmt" | "wet" => 0.0,
        "est" => -5.0,
        "edt" => -4.0,
        "cst" => -6.0,
        "cdt" => -5.0,
        "mst" => -7.0,
        "mdt" => -6.0,
        "pst" => -8.0,
        "pdt" => -7.0,
        "akst" => -9.0,
        "akdt" => -8.0,
        "hst" => -10.0,
        "ast" => -4.0,
        "adt" => -3.0,
        "nst" => -3.5,
        "ndt" => -2.5,
        "bst" | "cet" | "met" | "west" | "wat" => 1.0,
        "cest" | "eet" | "mest" | "sast" | "cat" => 2.0,
        "eest" | "msk" | "eat" => 3.0,
        "ist" => 5.5,
        "sgt" | "hkt" | "awst" => 8.0,
        "jst" | "kst" => 9.0,
        "acst" => 9.5,
        "aest" => 10.0,
        "acdt" => 10.5,
        "aedt" => 11.0,
        "nzst" => 12.0,
        "nzdt" => 13.0,
        _ if is_month_or_day(&lower) => return None,
        _ => 0.0,
    };
    Some((hours * 3600.0) as i64)
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

fn is_month_or_day(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    lower.len() >= 3
        && (MONTHS.iter().any(|m| lower.starts_with(m))
            || DAYS.iter().any(|d| lower.starts_with(d)))
}

fn find_date(text: &str) -> Option<NaiveDate> {
    static ISO: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\d{4})-(\d{1,2})-(\d{1,2})").unwrap());
    static NUMERIC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(\d{1,2})[./](\d{1,2})[./](\d{2,4})\b").unwrap());
    static MONTH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\b").unwrap()
    });
    static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'?\d+").unwrap());

    if let Some(c) = ISO.captures(text) {
        return NaiveDate::from_ymd_opt(c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?);
    }
    if let Some(c) = NUMERIC.captures(text) {
        let year = full_year(&c[3])?;
        return NaiveDate::from_ymd_opt(year, c[2].parse().ok()?, c[1].parse().ok()?);
    }
    let month = MONTH.captures(text)?;
    let month = MONTHS
        .iter()
        .position(|m| month[1].eq_ignore_ascii_case(m))? as u32
        + 1;
    // The year is the number that can't be a day of the month; otherwise
    // the day comes first.
    let numbers: Vec<&str> = NUMBER.find_iter(text).map(|m| m.as_str()).collect();
    let year_at = numbers.iter().position(|n| is_year(n));
    let (day, year) = match (year_at, numbers.as_slice()) {
        (Some(at), _) => (
            *numbers.iter().enumerate().find(|(i, _)| *i != at)?.1,
            numbers[at],
        ),
        (None, [day, year, ..]) => (*day, *year),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(full_year(year)?, month, day.parse().ok()?)
}

fn is_year(text: &str) -> bool {
    text.starts_with('\'') || text.len() > 2 || text.parse::<u32>().is_ok_and(|n| n > 31)
}

/// Two-digit years are 1969-2068, as Ruby's `Date` reads them.
fn full_year(text: &str) -> Option<i32> {
    let digits = text.trim_start_matches('\'');
    let year: i32 = digits.parse().ok()?;
    Some(if digits.len() <= 2 {
        if year >= 69 { 1900 + year } else { 2000 + year }
    } else {
        year
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expected values are what Ruby's `DateTime.parse` gives.
    #[test]
    fn parses_common_feed_dates() {
        let cases = [
            ("Fri, 09 May 2025 10:11:12 +0200", "2025-05-09T08:11:12Z"),
            ("Sep 18, 2024", "2024-09-18T00:00:00Z"),
            ("Friday, January 12, 2024 03:45 PM", "2024-01-12T15:45:00Z"),
            ("2024 Oct 05   10:11:12 PDT", "2024-10-05T17:11:12Z"),
            ("05.10.2024 10:11:12", "2024-10-05T10:11:12Z"),
            ("Thu, 01 May 2025 10:11:12 BST", "2025-05-01T09:11:12Z"),
            ("Fri, 9 May 2025 10:11:12 EST", "2025-05-09T15:11:12Z"),
            ("2024-10-05T10:11:12Z", "2024-10-05T10:11:12Z"),
            ("2024-10-05T10:11:12.123+02:00", "2024-10-05T08:11:12Z"),
            ("2024-10-05 10:11:12 +0000", "2024-10-05T10:11:12Z"),
            (
                "January, 05 Jan 2024 10:11:12 +0000",
                "2024-01-05T10:11:12Z",
            ),
            ("2024-10-05", "2024-10-05T00:00:00Z"),
            ("2024-10-05T10:11:12", "2024-10-05T10:11:12Z"),
            ("Mon, 05 Oct 2024 10:11 GMT", "2024-10-05T10:11:00Z"),
            ("05 Oct 2024", "2024-10-05T00:00:00Z"),
            ("Sat, 05 Oct 2024 10:11:12 CEST", "2024-10-05T08:11:12Z"),
            ("Sat, 05 Oct 2024 10:11:12 XYZ", "2024-10-05T10:11:12Z"),
            ("Sat, 5 Oct 24 10:11:12 GMT", "2024-10-05T10:11:12Z"),
            ("20241005101112", "2024-10-05T10:11:12Z"),
            ("Sat, 05 Oct 2024 10:11:12 +02:00", "2024-10-05T08:11:12Z"),
            ("Sat, 05 Oct 2024 10:11:12 UT", "2024-10-05T10:11:12Z"),
            ("Sat, 05 Oct 2024 10:11:12 Z", "2024-10-05T10:11:12Z"),
            ("10/05/2024", "2024-05-10T00:00:00Z"),
            ("Sat Oct 05 10:11:12 UTC 2024", "2024-10-05T10:11:12Z"),
            ("2024-10-05T10:11:12+0200", "2024-10-05T08:11:12Z"),
            ("Oct 5 2024 3pm", "2024-10-05T15:00:00Z"),
        ];
        for (input, expected) in cases {
            let actual = parse(input).map(|time| time.format("%Y-%m-%dT%H:%M:%SZ").to_string());
            assert_eq!(actual.as_deref(), Some(expected), "{input}");
        }
    }

    #[test]
    fn rejects_invalid_dates() {
        for input in [
            "",
            "garbage",
            "Sat, 05 Oct 2024 25:11:12 GMT",
            "2024-02-30T00:00:00Z",
        ] {
            assert_eq!(parse(input), None, "{input}");
        }
    }
}
