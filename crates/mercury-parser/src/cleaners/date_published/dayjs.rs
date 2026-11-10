//! Emulation of the dayjs (1.11) behavior the cleaner uses: default string
//! parsing, the `customParseFormat` plugin (en locale), the `timezone`
//! plugin's `dayjs.tz(input, format, tz)` and `.tz(tz, true)`, and
//! `subtract` for relative dates.
//!
//! Instants are JS time values (`f64` ms, `NaN` = invalid). Local time is UTC.

use std::str::FromStr;
use std::sync::LazyLock;

use chrono::{DateTime, Offset, TimeZone};
use chrono_tz::Tz;
use regex::Regex;

use super::js_date::{self, MS_PER_MINUTE, MS_PER_SECOND, is_js_whitespace};

/// dayjs's `REGEX_PARSE`, used by `dayjs(string)`.
static REGEX_PARSE: LazyLock<Regex> = LazyLock::new(|| {
    let ws = super::JS_WS;
    Regex::new(&format!(
        r"^([0-9]{{4}})[-/]?([0-9]{{1,2}})?[-/]?([0-9]{{0,2}})[Tt{ws}]*([0-9]{{1,2}})?:?([0-9]{{1,2}})?:?([0-9]{{1,2}})?[.:]?([0-9]+)?$"
    ))
    .unwrap()
});

const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `dayjs(string)` without a format.
pub fn parse(input: &str) -> f64 {
    if input.ends_with(['Z', 'z']) {
        return js_date::parse(input);
    }
    let Some(caps) = REGEX_PARSE.captures(input) else {
        return js_date::parse(input);
    };
    let num = |i: usize| caps.get(i).map(|m| js_number(m.as_str()));
    // `d[2] - 1 || 0`, `d[3] || 1`, `d[4] || 0`, ...
    let month = num(2).map_or(0.0, |m| or_default(m - 1.0, 0.0));
    let day = caps
        .get(3)
        .filter(|m| !m.as_str().is_empty())
        .map_or(1.0, |m| js_number(m.as_str()));
    let hour = num(4).map_or(0.0, |v| or_default(v, 0.0));
    let minute = num(5).map_or(0.0, |v| or_default(v, 0.0));
    let second = num(6).map_or(0.0, |v| or_default(v, 0.0));
    let ms = caps.get(7).map_or(0.0, |m| {
        let s = m.as_str();
        js_number(&s[..s.len().min(3)])
    });
    js_date::make_utc(
        num(1).unwrap_or(f64::NAN),
        month,
        day,
        hour,
        minute,
        second,
        ms,
    )
}

/// `dayjs(input, format)` (or `dayjs.utc(input, format)` when `utc`) with the
/// `customParseFormat` plugin and the en locale. Local time is UTC, so both
/// only differ in how the week token and zones are treated.
pub fn parse_with_format(input: &str, format: &str, utc: bool, now: f64) -> f64 {
    parse_formatted_input(input, format, utc, now).unwrap_or(f64::NAN)
}

/// `dayjs(instant).tz(timezone, true)`: keep the wall-clock time and
/// reinterpret it in `timezone`. `None` when JS would throw (unknown zone).
pub fn tz_keep_local_time(t: f64, timezone: &str) -> Option<f64> {
    let tz = parse_timezone(timezone)?;
    if t.is_nan() {
        return Some(f64::NAN);
    }
    // `new Date(date.toLocaleString('en-US', { timeZone }))`: the wall clock
    // in `timezone`, formatted like "1/2/2020, 5:00:00 AM" and parsed back
    // by the legacy parser (so two-digit years shift centuries, like JS).
    let wall = wall_clock(t, tz)?;
    let hour12 = if wall.hour % 12 == 0 {
        12
    } else {
        wall.hour % 12
    };
    let meridiem = if wall.hour < 12 { "AM" } else { "PM" };
    let target = js_date::parse(&format!(
        "{}/{}/{}, {hour12}:{:02}:{:02} {meridiem}",
        wall.month, wall.day, wall.year, wall.minute, wall.second
    ));
    let diff = js_round((t - target) / MS_PER_MINUTE);
    let offset = -diff;
    // `!Number(offset)`: zero or NaN switches to UTC mode, keeping `t`.
    if offset == 0.0 || offset.is_nan() {
        return Some(t);
    }
    let offset = if offset.abs() <= 16.0 {
        offset * 60.0
    } else {
        offset
    };
    // `d(target).$set(MS, this.$ms).utcOffset(offset, true)` followed by
    // `.add(-offset, 'minute')`; the offset ends up applied twice.
    let local_ms = t.rem_euclid(MS_PER_SECOND);
    let shifted = js_date::time_clip(target + local_ms - offset * MS_PER_MINUTE);
    Some(js_date::time_clip(shifted - offset * MS_PER_MINUTE))
}

/// `dayjs.tz(input, format, timezone)`. Returns `NaN` both for invalid
/// results and where dayjs throws (which the cleaner catches).
pub fn tz_parse(input: &str, format: &str, timezone: &str, now: f64) -> f64 {
    let Some(tz) = parse_timezone(timezone) else {
        return f64::NAN;
    };
    let Some(previous_offset) = tz_offset(now, tz) else {
        return f64::NAN;
    };
    let local_ts = parse_with_format(input, format, true, now);
    let Some((target_ts, target_offset)) = fix_offset(local_ts, previous_offset, tz) else {
        return f64::NAN;
    };
    // `d(targetTs).utcOffset(targetOffset)` keeps the instant.
    let t = js_date::time_clip(target_ts);
    let offset = if target_offset.abs() <= 16.0 {
        target_offset * 60.0
    } else {
        target_offset
    };
    if offset == 0.0 || t.is_nan() {
        return t;
    }
    let shifted = js_date::time_clip(t + offset * MS_PER_MINUTE);
    js_date::time_clip(shifted - offset * MS_PER_MINUTE)
}

/// `dayjs(now).subtract(amount, unit)` where `amount` is a digit string and
/// `unit` one of second(s) ... year(s), any case.
pub fn subtract(now: f64, amount: &str, unit: &str) -> f64 {
    let number = -js_number(amount);
    let unit = unit.to_ascii_lowercase();
    let unit = unit.strip_suffix('s').unwrap_or(&unit);
    let (year, month, day, time) = js_date::fields(now);
    let step = match unit {
        "second" => MS_PER_SECOND,
        "minute" => MS_PER_MINUTE,
        "hour" => 3_600_000.0,
        "day" | "week" => {
            let n = if unit == "week" { 7.0 } else { 1.0 };
            let date = day + js_round(n * number);
            return js_date::time_clip(js_date::make_date(
                js_date::make_day(year, month, date),
                time,
            ));
        }
        "month" | "year" => {
            let (year, month) = if unit == "month" {
                (year, month + number)
            } else {
                (year + number, month)
            };
            let first = js_date::time_clip(js_date::make_date(
                js_date::make_day(year, month, 1.0),
                time,
            ));
            if first.is_nan() {
                return f64::NAN;
            }
            let (y, m, _, _) = js_date::fields(first);
            let days_in_month = js_date::make_day(y, m + 1.0, 1.0) - js_date::make_day(y, m, 1.0);
            return js_date::time_clip(js_date::make_date(
                js_date::make_day(y, m, day.min(days_in_month)),
                time,
            ));
        }
        _ => 1.0,
    };
    js_date::time_clip(now + number * step)
}

#[derive(Default)]
struct ParsedTime {
    year: Option<f64>,
    month: Option<f64>,
    day: Option<f64>,
    /// `Do` stores the matched digits as a string, which is always truthy.
    day_is_string: bool,
    hours: Option<f64>,
    minutes: Option<f64>,
    seconds: Option<f64>,
    milliseconds: Option<f64>,
    afternoon: Option<bool>,
    zone_offset: Option<f64>,
    week: Option<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Matcher {
    Digits1,
    Digits2,
    Digits3,
    Digits4,
    Digits1To2,
    Signed,
    Offset,
    Word,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Field {
    Afternoon { lower: bool },
    Quarter,
    Millis(f64),
    Seconds,
    Minutes,
    Hours,
    Day,
    DayOrdinal,
    Week,
    Month,
    MonthShortName,
    MonthName,
    Year,
    TwoDigitYear,
    Zone,
}

enum FormatPart {
    Literal(usize),
    Token(Matcher, Field),
}

// `parseFormattedInput`; `None` corresponds to the plugin's catch block.
fn parse_formatted_input(input: &str, format: &str, utc: bool, now: f64) -> Option<f64> {
    if format == "x" || format == "X" {
        let factor = if format == "X" { 1000.0 } else { 1.0 };
        return Some(js_date::time_clip(factor * js_number(input)));
    }
    let parts = make_parser(format)?;
    let mut input: Vec<u16> = input.encode_utf16().collect();
    let mut time = ParsedTime::default();
    let mut start = 0usize;
    for part in &parts {
        match *part {
            FormatPart::Literal(len) => start += len,
            FormatPart::Token(matcher, field) => {
                let rest = input.get(start..).unwrap_or(&[]);
                let (from, to) = find_match(matcher, rest)?;
                let value = rest[from..to].to_vec();
                apply_field(&mut time, field, &value)?;
                // `input = input.replace(value, '')` removes the first
                // occurrence anywhere in the input.
                if let Some(pos) = find_sub(&input, &value) {
                    input.drain(pos..pos + value.len());
                }
            }
        }
    }
    correct_hours(&mut time);

    let (now_year, now_month, now_day, _) = js_date::fields(now);
    let year_truthy = time.year.is_some_and(truthy);
    let month_truthy = time.month.is_some_and(truthy);
    let day_truthy = time.day_is_string || time.day.is_some_and(truthy);
    let d = if day_truthy {
        time.day.unwrap_or(f64::NAN)
    } else if !year_truthy && !month_truthy {
        now_day
    } else {
        1.0
    };
    let y = if year_truthy {
        time.year.unwrap()
    } else {
        now_year
    };
    let mut m = 0.0;
    if !year_truthy || month_truthy {
        m = match time.month {
            Some(month) if month > 0.0 => month - 1.0,
            _ => now_month,
        };
    }
    let or0 = |v: Option<f64>| v.filter(|&x| truthy(x)).unwrap_or(0.0);
    let (h, min, s, ms) = (
        or0(time.hours),
        or0(time.minutes),
        or0(time.seconds),
        or0(time.milliseconds),
    );
    if let Some(offset) = time.zone_offset {
        return Some(js_date::make_utc(
            y,
            m,
            d,
            h,
            min,
            s,
            ms + offset * MS_PER_MINUTE,
        ));
    }
    if !utc && time.week.is_some_and(truthy) {
        // `dayjs(date).week(...)` needs the weekOfYear plugin, which isn't
        // loaded, so this throws.
        return None;
    }
    Some(js_date::make_utc(y, m, d, h, min, s, ms))
}

fn make_parser(format: &str) -> Option<Vec<FormatPart>> {
    let format: Vec<u16> = format.encode_utf16().collect();
    // The localizedFormat expansion throws on L/LT/l... tokens because the en
    // locale has no `formats`.
    if has_localized_token(&format) {
        return None;
    }
    let mut parts = Vec::new();
    let mut i = 0;
    while i < format.len() {
        if let Some(len) = bracket_len(&format[i..]) {
            // `[...]` literal, with the brackets stripped.
            parts.push(FormatPart::Literal(len - 2));
            i += len;
            continue;
        }
        let sep = format[i..].iter().take_while(|&&c| is_separator(c)).count();
        if sep > 0 {
            parts.push(FormatPart::Literal(sep));
            i += sep;
            continue;
        }
        match format_token(&format[i..]) {
            Some((len, Some((matcher, field)))) => {
                parts.push(FormatPart::Token(matcher, field));
                i += len;
            }
            Some((len, None)) => {
                parts.push(FormatPart::Literal(len));
                i += len;
            }
            None => i += 1,
        }
    }
    // `format.match(...)` returning null makes `array.length` throw.
    (!parts.is_empty()).then_some(parts)
}

// Length of a `\[[^[]*\]` match at the start of `s`.
fn bracket_len(s: &[u16]) -> Option<usize> {
    if s.first() != Some(&u16::from(b'[')) {
        return None;
    }
    let end = s[1..]
        .iter()
        .position(|&c| c == u16::from(b'['))
        .map_or(s.len(), |p| p + 1);
    s[1..end]
        .iter()
        .rposition(|&c| c == u16::from(b']'))
        .map(|p| p + 2)
}

// `(\[[^\]]+])|(LTS?|l{1,4}|L{1,4})` from localizedFormat's `u()`.
fn has_localized_token(s: &[u16]) -> bool {
    let mut i = 0;
    while i < s.len() {
        if s[i] == u16::from(b'[')
            && let Some(close) = s[i + 1..].iter().position(|&c| c == u16::from(b']'))
            && close > 0
        {
            i += close + 2;
            continue;
        }
        if s[i] == u16::from(b'L') || s[i] == u16::from(b'l') {
            return true;
        }
        i += 1;
    }
    false
}

fn is_separator(c: u16) -> bool {
    matches!(c, 0x2D | 0x5F | 0x3A | 0x2F | 0x2E | 0x2C | 0x28 | 0x29)
        || is_js_whitespace(u32::from(c))
}

// Matches `A|a|Q|YYYY|YY?|ww?|MM?M?M?|Do|DD?|hh?|HH?|mm?|ss?|S{1,3}|z|ZZ?` at the
// start of `s`, returning its length and parser (`None` for tokens without
// one, which become literals).
fn format_token(s: &[u16]) -> Option<(usize, Option<(Matcher, Field)>)> {
    let c = char::from_u32(u32::from(*s.first()?))?;
    let run = |max: usize| s.iter().take(max).take_while(|&&x| x == s[0]).count();
    let token = |len, m, f| Some((len, Some((m, f))));
    use Field::*;
    use Matcher::*;
    match c {
        'A' => token(1, Word, Afternoon { lower: false }),
        'a' => token(1, Word, Afternoon { lower: true }),
        'Q' => token(1, Digits1, Quarter),
        'Y' => match run(4) {
            4 => token(4, Digits4, Year),
            2 | 3 => token(2, Digits2, TwoDigitYear),
            _ => token(1, Signed, Year),
        },
        'w' => match run(2) {
            2 => token(2, Digits2, Week),
            _ => token(1, Digits1To2, Week),
        },
        'M' => match run(4) {
            4 => token(4, Word, MonthName),
            3 => token(3, Word, MonthShortName),
            2 => token(2, Digits2, Month),
            _ => token(1, Digits1To2, Month),
        },
        'D' if s.get(1) == Some(&u16::from(b'o')) => token(2, Word, DayOrdinal),
        'D' => match run(2) {
            2 => token(2, Digits2, Day),
            _ => token(1, Digits1To2, Day),
        },
        'h' | 'H' => token(run(2), Digits1To2, Hours),
        'm' => token(run(2), Digits1To2, Minutes),
        's' => token(run(2), Digits1To2, Seconds),
        'S' => match run(3) {
            3 => token(3, Digits3, Millis(1.0)),
            2 => token(2, Digits2, Millis(10.0)),
            _ => token(1, Digits1, Millis(100.0)),
        },
        'z' => Some((1, None)),
        'Z' => token(run(2), Offset, Zone),
        _ => None,
    }
}

// First match of the token's regex within `s`, as a [from, to) range.
fn find_match(matcher: Matcher, s: &[u16]) -> Option<(usize, usize)> {
    let digit = |i: usize| s.get(i).is_some_and(|&c| (0x30..=0x39).contains(&c));
    let fixed = |n: usize| {
        (0..s.len())
            .find(|&i| (i..i + n).all(digit))
            .map(|i| (i, i + n))
    };
    match matcher {
        Matcher::Digits1 => fixed(1),
        Matcher::Digits2 => fixed(2),
        Matcher::Digits3 => fixed(3),
        Matcher::Digits4 => fixed(4),
        Matcher::Digits1To2 => fixed(1).map(|(i, _)| (i, if digit(i + 1) { i + 2 } else { i + 1 })),
        Matcher::Signed => (0..s.len()).find_map(|i| {
            let signed = matches!(s[i], 0x2B | 0x2D) && digit(i + 1);
            let digits_from = if signed { i + 1 } else { i };
            (signed || digit(i)).then(|| {
                let end = (digits_from..).find(|&j| !digit(j)).unwrap();
                (i, end)
            })
        }),
        Matcher::Offset => (0..s.len()).find_map(|i| {
            if matches!(s[i], 0x2B | 0x2D) && digit(i + 1) && digit(i + 2) {
                let mut end = i + 3;
                let colon = s.get(end) == Some(&u16::from(b':'));
                let after = if colon { end + 1 } else { end };
                if digit(after) && digit(after + 1) {
                    end = after + 2;
                } else if colon {
                    end += 1;
                }
                Some((i, end))
            } else {
                (s[i] == u16::from(b'Z')).then_some((i, i + 1))
            }
        }),
        Matcher::Word => {
            let is_word = |c: u16| {
                !matches!(c, 0x2D | 0x5F | 0x3A | 0x2F | 0x2C | 0x28 | 0x29)
                    && !is_js_whitespace(u32::from(c))
                    && !(0x30..=0x39).contains(&c)
            };
            (0..s.len()).find_map(|i| {
                let j = (i..).find(|&j| !digit(j)).unwrap();
                if j < s.len() && is_word(s[j]) {
                    let k = (j..).find(|&k| k >= s.len() || !is_word(s[k])).unwrap();
                    Some((i, k))
                } else {
                    None
                }
            })
        }
    }
}

fn apply_field(time: &mut ParsedTime, field: Field, value: &[u16]) -> Option<()> {
    let text = String::from_utf16_lossy(value);
    let number = || js_number(&text);
    match field {
        Field::Afternoon { lower } => {
            time.afternoon = Some(text == if lower { "pm" } else { "PM" });
        }
        Field::Quarter => time.month = Some((number() - 1.0) * 3.0 + 1.0),
        Field::Millis(factor) => time.milliseconds = Some(number() * factor),
        Field::Seconds => time.seconds = Some(number()),
        Field::Minutes => time.minutes = Some(number()),
        Field::Hours => time.hours = Some(number()),
        Field::Day => {
            time.day = Some(number());
            time.day_is_string = false;
        }
        Field::DayOrdinal => {
            let digits: String = text
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(char::is_ascii_digit)
                .collect();
            if digits.is_empty() {
                return None;
            }
            time.day = Some(js_number(&digits));
            time.day_is_string = true;
            if let Some(n) = (1..=31).find(|&n| en_ordinal(n) == text) {
                time.day = Some(f64::from(n));
                time.day_is_string = false;
            }
        }
        Field::Week => time.week = Some(number()),
        Field::Month => time.month = Some(number()),
        Field::MonthShortName | Field::MonthName => {
            let index = if field == Field::MonthName {
                EN_MONTHS.iter().position(|m| *m == text)
            } else {
                EN_MONTHS.iter().position(|m| m[..3] == text)
            }?;
            time.month = Some((index + 1) as f64);
        }
        Field::Year => time.year = Some(number()),
        Field::TwoDigitYear => {
            let n = number();
            time.year = Some(n + if n > 68.0 { 1900.0 } else { 2000.0 });
        }
        Field::Zone => time.zone_offset = Some(zone_offset_from_string(&text)),
    }
    Some(())
}

// The plugin's `offsetFromString`: minutes to add to reach UTC.
fn zone_offset_from_string(s: &str) -> f64 {
    if s.is_empty() || s == "Z" {
        return 0.0;
    }
    let b = s.as_bytes();
    let mut parts: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'+' || b[i] == b'-' {
            parts.push(&s[i..=i]);
            i += 1;
        } else if i + 1 < b.len() && b[i].is_ascii_digit() && b[i + 1].is_ascii_digit() {
            parts.push(&s[i..i + 2]);
            i += 2;
        } else {
            i += 1;
        }
    }
    let hours = parts.get(1).map_or(f64::NAN, |p| js_number(p));
    let minutes = parts.get(2).map_or(0.0, |p| or_default(js_number(p), 0.0));
    let total = hours * 60.0 + minutes;
    if total == 0.0 {
        0.0
    } else if parts.first() == Some(&"+") {
        -total
    } else {
        total
    }
}

fn en_ordinal(n: u32) -> String {
    let suffixes = ["th", "st", "nd", "rd"];
    let v = n % 100;
    let suffix = (v as i64 - 20)
        .checked_rem(10)
        .filter(|r| (1..=3).contains(r))
        .map(|r| suffixes[r as usize])
        .or_else(|| suffixes.get(v as usize).copied())
        .unwrap_or("th");
    format!("{n}{suffix}")
}

fn correct_hours(time: &mut ParsedTime) {
    let Some(afternoon) = time.afternoon.take() else {
        return;
    };
    let Some(hours) = time.hours else {
        return;
    };
    if afternoon {
        if hours < 12.0 {
            time.hours = Some(hours + 12.0);
        }
    } else if hours == 12.0 {
        time.hours = Some(0.0);
    }
}

fn find_sub(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_timezone(name: &str) -> Option<Tz> {
    Tz::from_str(name)
        .or_else(|_| Tz::from_str_insensitive(name))
        .ok()
}

// Offset (seconds east of UTC) of `tz` at the whole second `secs`.
fn offset_seconds(secs: i64, tz: Tz) -> Option<i64> {
    let secs = secs - equivalent_year_shift(secs) * 86_400;
    let utc = DateTime::from_timestamp(secs, 0)?.naive_utc();
    Some(i64::from(
        tz.offset_from_utc_datetime(&utc).fix().local_minus_utc(),
    ))
}

// chrono-tz only has transitions through 2099, while ICU keeps applying the
// zone's final rules. Later years are looked up in the 2040-2099 year with
// the same calendar (leap-ness and weekday of January 1st); very early years
// are moved by whole 400-year cycles (they are all LMT anyway).
fn equivalent_year_shift(secs: i64) -> i64 {
    let days = secs.div_euclid(86_400);
    let (year, _, _) = js_date::civil_from_days(days);
    let jan1 = |y: i64| js_date::days_from_civil(y, 1, 1);
    let weekday = |d: i64| (d + 4).rem_euclid(7);
    let is_leap = |y: i64| (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    if year >= 2100 {
        let target = (2040..2100)
            .find(|&y| is_leap(y) == is_leap(year) && weekday(jan1(y)) == weekday(jan1(year)))
            .unwrap_or(year);
        jan1(year) - jan1(target)
    } else if year < 1400 {
        let cycles = (1400 - year + 399) / 400;
        -cycles * 146_097
    } else {
        0
    }
}

struct WallClock {
    /// Era year, as `Intl.DateTimeFormat` prints it (1 BC is 1).
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

// The fields `Intl.DateTimeFormat('en-US', { timeZone })` shows for `t`.
fn wall_clock(t: f64, tz: Tz) -> Option<WallClock> {
    let secs = (t / MS_PER_SECOND).floor() as i64;
    let wall = secs + offset_seconds(secs, tz)?;
    let (year, month, day) = js_date::civil_from_days(wall.div_euclid(86_400));
    let rem = wall.rem_euclid(86_400);
    Some(WallClock {
        year: if year <= 0 { 1 - year } else { year },
        month,
        day,
        hour: rem / 3600,
        minute: rem / 60 % 60,
        second: rem % 60,
    })
}

// The timezone plugin's `tzOffset`, in minutes. `None` where Intl throws.
fn tz_offset(timestamp: f64, tz: Tz) -> Option<f64> {
    if timestamp.is_nan() {
        return None;
    }
    let w = wall_clock(timestamp, tz)?;
    let utc_string = format!(
        "{}-{}-{} {}:{}:{}:000",
        w.year, w.month, w.day, w.hour, w.minute, w.second
    );
    let utc_ts = parse(&utc_string);
    let as_ts = timestamp - timestamp % MS_PER_SECOND;
    Some((utc_ts - as_ts) / MS_PER_MINUTE)
}

// The timezone plugin's `fixOffset` (after luxon).
fn fix_offset(local_ts: f64, o0: f64, tz: Tz) -> Option<(f64, f64)> {
    let mut utc_guess = local_ts - o0 * MS_PER_MINUTE;
    let o2 = tz_offset(utc_guess, tz)?;
    if o0 == o2 {
        return Some((utc_guess, o0));
    }
    utc_guess -= (o2 - o0) * MS_PER_MINUTE;
    let o3 = tz_offset(utc_guess, tz)?;
    if o2 == o3 {
        return Some((utc_guess, o2));
    }
    Some((local_ts - o2.min(o3) * MS_PER_MINUTE, o2.max(o3)))
}

fn truthy(x: f64) -> bool {
    x != 0.0 && !x.is_nan()
}

fn or_default(x: f64, default: f64) -> f64 {
    if truthy(x) { x } else { default }
}

// JS `Math.round`.
fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// JS `Number(string)` for the digit-ish strings the cleaner produces.
fn js_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| is_js_whitespace(c as u32));
    if t.is_empty() {
        return 0.0;
    }
    let radix = |prefix: [&str; 2], radix| {
        prefix.iter().find_map(|p| t.strip_prefix(p)).map(|digits| {
            if digits.is_empty() {
                return f64::NAN;
            }
            digits
                .chars()
                .try_fold(0.0, |acc, c| {
                    c.to_digit(radix)
                        .map(|d| acc * f64::from(radix) + f64::from(d))
                })
                .unwrap_or(f64::NAN)
        })
    };
    if let Some(v) = radix(["0x", "0X"], 16)
        .or_else(|| radix(["0o", "0O"], 8))
        .or_else(|| radix(["0b", "0B"], 2))
    {
        return v;
    }
    let unsigned = t.strip_prefix(['+', '-']).unwrap_or(t);
    if unsigned == "Infinity" {
        return if t.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let valid = !unsigned.is_empty()
        && unsigned
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'));
    if !valid {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals() {
        assert_eq!(en_ordinal(1), "1st");
        assert_eq!(en_ordinal(2), "2nd");
        assert_eq!(en_ordinal(3), "3rd");
        assert_eq!(en_ordinal(4), "4th");
        assert_eq!(en_ordinal(11), "11th");
        assert_eq!(en_ordinal(12), "12th");
        assert_eq!(en_ordinal(21), "21st");
        assert_eq!(en_ordinal(23), "23rd");
        assert_eq!(en_ordinal(30), "30th");
    }

    #[test]
    fn js_round_matches() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-2.6), -3.0);
    }
}
