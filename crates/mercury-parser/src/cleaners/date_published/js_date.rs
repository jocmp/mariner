//! Emulation of the pieces of JavaScript's `Date` that the cleaner relies on:
//! V8's `Date.parse` (ES5 ISO parsing with a fallback to V8's legacy
//! heuristic parser, see `src/date/dateparser*.{h,cc}`), the spec's
//! `MakeDay`/`MakeTime`/`TimeClip` arithmetic, and `toISOString`.
//!
//! Times are `f64` milliseconds since the epoch, with `NaN` standing in for
//! an Invalid Date, exactly as in JS. Local time is treated as UTC.

pub const MS_PER_SECOND: f64 = 1000.0;
pub const MS_PER_MINUTE: f64 = 60_000.0;
pub const MS_PER_DAY: f64 = 86_400_000.0;

const MAX_TIME_MS: f64 = 8.64e15;

/// `Date.parse(s)` as implemented by V8.
pub fn parse(s: &str) -> f64 {
    let units: Vec<u16> = s.encode_utf16().collect();
    let Some(out) = parse_components(&units) else {
        return f64::NAN;
    };
    let day = make_day(out.year, out.month, out.day);
    let time = make_time(out.hour, out.minute, out.second, out.millisecond);
    let mut date = make_date(day, time);
    if let Some(offset) = out.utc_offset {
        date -= offset * MS_PER_SECOND;
    }
    time_clip(date)
}

/// `Date.UTC(y, m, d, h, min, s, ms)` (and, with local time being UTC, the
/// `new Date(y, m, ...)` constructor).
pub fn make_utc(y: f64, m: f64, d: f64, h: f64, min: f64, s: f64, ms: f64) -> f64 {
    let mut year = y;
    if !year.is_nan() {
        let yi = to_integer(year);
        if (0.0..=99.0).contains(&yi) {
            year = 1900.0 + yi;
        }
    }
    time_clip(make_date(make_day(year, m, d), make_time(h, min, s, ms)))
}

/// `Date.prototype.toISOString`, or `None` where JS would throw a RangeError.
pub fn to_iso_string(t: f64) -> Option<String> {
    if !t.is_finite() {
        return None;
    }
    let t = t as i64;
    let days = t.div_euclid(MS_PER_DAY as i64);
    let ms_in_day = t.rem_euclid(MS_PER_DAY as i64);
    let (year, month, day) = civil_from_days(days);
    let year_str = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else if year < 0 {
        format!("-{:06}", -year)
    } else {
        format!("+{year:06}")
    };
    Some(format!(
        "{year_str}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        ms_in_day / 3_600_000,
        ms_in_day / 60_000 % 60,
        ms_in_day / 1000 % 60,
        ms_in_day % 1000
    ))
}

/// Year, zero-based month, day of month and time within the day for a valid
/// time value (the `YearFromTime`, `MonthFromTime`, ... spec operations).
pub fn fields(t: f64) -> (f64, f64, f64, f64) {
    let t = t as i64;
    let days = t.div_euclid(MS_PER_DAY as i64);
    let (year, month, day) = civil_from_days(days);
    (
        year as f64,
        (month - 1) as f64,
        day as f64,
        t.rem_euclid(MS_PER_DAY as i64) as f64,
    )
}

/// Spec `MakeDay`, with V8's year/month range limits.
pub fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !year.is_finite() || !month.is_finite() || !date.is_finite() {
        return f64::NAN;
    }
    let (year, month, date) = (to_integer(year), to_integer(month), to_integer(date));
    if !(-1_000_000.0..=1_000_000.0).contains(&year)
        || !(-10_000_000.0..=10_000_000.0).contains(&month)
    {
        return f64::NAN;
    }
    let (year, month) = (year as i64, month as i64);
    let y = year + month.div_euclid(12);
    let m = month.rem_euclid(12);
    days_from_civil(y, m + 1, 1) as f64 + date - 1.0
}

/// Spec `MakeTime`.
pub fn make_time(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    if !hour.is_finite() || !min.is_finite() || !sec.is_finite() || !ms.is_finite() {
        return f64::NAN;
    }
    to_integer(hour) * 3_600_000.0
        + to_integer(min) * MS_PER_MINUTE
        + to_integer(sec) * MS_PER_SECOND
        + to_integer(ms)
}

/// Spec `MakeDate`.
pub fn make_date(day: f64, time: f64) -> f64 {
    if !day.is_finite() || !time.is_finite() {
        return f64::NAN;
    }
    day * MS_PER_DAY + time
}

/// Spec `TimeClip`.
pub fn time_clip(t: f64) -> f64 {
    if !t.is_finite() || t.abs() > MAX_TIME_MS {
        return f64::NAN;
    }
    to_integer(t) + 0.0
}

/// JS `\s` / `String.prototype.trim` whitespace (WhiteSpace + LineTerminator).
pub fn is_js_whitespace(c: u32) -> bool {
    is_white_space(c) || matches!(c, 0x0A | 0x0D | 0x2028 | 0x2029)
}

/// Days since the epoch for a proleptic Gregorian date (1-based month).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of `days_from_civil`: (year, month 1-12, day).
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Spec `ToIntegerOrInfinity` for finite values.
fn to_integer(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.trunc() + 0.0 }
}

// ES WhiteSpace: TAB, VT, FF, ZWNBSP and the Zs category.
fn is_white_space(c: u32) -> bool {
    matches!(
        c,
        0x09 | 0x0B | 0x0C | 0x20 | 0xA0 | 0x1680 | 0x2000
            ..=0x200A | 0x202F | 0x205F | 0x3000 | 0xFEFF
    )
}

// ---------------------------------------------------------------------------
// V8 DateParser
// ---------------------------------------------------------------------------

const NONE: i32 = i32::MAX;
const MAX_SIGNIFICANT_DIGITS: i32 = 9;
const SMI_MAX: u32 = i32::MAX as u32;

struct Output {
    year: f64,
    month: f64,
    day: f64,
    hour: f64,
    minute: f64,
    second: f64,
    millisecond: f64,
    /// Seconds east of UTC; `None` means local time.
    utc_offset: Option<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeywordType {
    Invalid,
    MonthName,
    TimeZoneName,
    TimeSeparator,
    AmPm,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Invalid,
    Unknown,
    WhiteSpace,
    Number,
    Symbol,
    EndOfInput,
    Keyword(KeywordType),
}

#[derive(Clone, Copy, Debug)]
struct Token {
    tag: Tag,
    length: i32,
    value: i32,
}

impl Token {
    fn new(tag: Tag, length: i32, value: i32) -> Self {
        Token { tag, length, value }
    }
    fn invalid() -> Self {
        Token::new(Tag::Invalid, 0, 0)
    }
    fn is_invalid(&self) -> bool {
        self.tag == Tag::Invalid
    }
    fn is_number(&self) -> bool {
        self.tag == Tag::Number
    }
    fn is_symbol(&self, c: u8) -> bool {
        self.tag == Tag::Symbol && self.value == i32::from(c)
    }
    fn is_white_space(&self) -> bool {
        self.tag == Tag::WhiteSpace
    }
    fn is_end_of_input(&self) -> bool {
        self.tag == Tag::EndOfInput
    }
    fn is_keyword(&self) -> bool {
        matches!(self.tag, Tag::Keyword(_))
    }
    fn is_keyword_type(&self, t: KeywordType) -> bool {
        self.tag == Tag::Keyword(t)
    }
    fn is_keyword_z(&self) -> bool {
        self.tag == Tag::Keyword(KeywordType::TimeZoneName) && self.length == 1 && self.value == 0
    }
    fn is_fixed_length_number(&self, n: i32) -> bool {
        self.is_number() && self.length == n
    }
    fn is_ascii_sign(&self) -> bool {
        self.tag == Tag::Symbol && (self.value == i32::from(b'+') || self.value == i32::from(b'-'))
    }
    fn ascii_sign(&self) -> i32 {
        44 - self.value
    }
}

struct InputReader<'a> {
    buffer: &'a [u16],
    index: usize,
    ch: u32,
}

impl<'a> InputReader<'a> {
    fn new(buffer: &'a [u16]) -> Self {
        let mut reader = InputReader {
            buffer,
            index: 0,
            ch: 0,
        };
        reader.next();
        reader
    }

    fn position(&self) -> i32 {
        self.index as i32
    }

    fn next(&mut self) {
        self.ch = self.buffer.get(self.index).map_or(0, |&c| u32::from(c));
        self.index += 1;
    }

    fn read_unsigned_numeral(&mut self) -> i32 {
        let mut n = 0i32;
        let mut i = 0;
        while self.ch == u32::from(b'0') {
            self.next();
        }
        while self.is_ascii_digit() {
            if i < MAX_SIGNIFICANT_DIGITS {
                n = n * 10 + (self.ch - u32::from(b'0')) as i32;
            }
            i += 1;
            self.next();
        }
        n
    }

    fn read_word(&mut self, prefix: &mut [u32; 3]) -> i32 {
        let mut len = 0;
        while self.is_ascii_alpha_or_above() && !is_white_space(self.ch) {
            if len < 3 {
                prefix[len] = self.ch | 0x20;
            }
            len += 1;
            self.next();
        }
        len as i32
    }

    fn skip(&mut self, c: u8) -> bool {
        if self.ch == u32::from(c) {
            self.next();
            true
        } else {
            false
        }
    }

    fn skip_white_space(&mut self) -> bool {
        if is_js_whitespace(self.ch) {
            self.next();
            true
        } else {
            false
        }
    }

    fn skip_parentheses(&mut self) -> bool {
        if self.ch != u32::from(b'(') {
            return false;
        }
        let mut balance = 0;
        loop {
            if self.ch == u32::from(b')') {
                balance -= 1;
            } else if self.ch == u32::from(b'(') {
                balance += 1;
            }
            self.next();
            if balance <= 0 || self.ch == 0 {
                break;
            }
        }
        true
    }

    fn is_end(&self) -> bool {
        self.ch == 0
    }

    fn is_ascii_digit(&self) -> bool {
        (u32::from(b'0')..=u32::from(b'9')).contains(&self.ch)
    }

    fn is_ascii_alpha_or_above(&self) -> bool {
        self.ch >= u32::from(b'A')
    }
}

struct Tokenizer<'a> {
    input: InputReader<'a>,
    next: Token,
}

impl<'a> Tokenizer<'a> {
    fn new(buffer: &'a [u16]) -> Self {
        let mut input = InputReader::new(buffer);
        let next = scan(&mut input);
        Tokenizer { input, next }
    }

    fn next(&mut self) -> Token {
        let result = self.next;
        self.next = scan(&mut self.input);
        result
    }

    fn peek(&self) -> Token {
        self.next
    }

    fn skip_symbol(&mut self, c: u8) -> bool {
        if self.next.is_symbol(c) {
            self.next = scan(&mut self.input);
            true
        } else {
            false
        }
    }
}

fn scan(input: &mut InputReader) -> Token {
    let pre_pos = input.position();
    if input.is_end() {
        return Token::new(Tag::EndOfInput, 0, 0);
    }
    if input.is_ascii_digit() {
        let n = input.read_unsigned_numeral();
        return Token::new(Tag::Number, input.position() - pre_pos, n);
    }
    for c in *b":-+.)" {
        if input.skip(c) {
            return Token::new(Tag::Symbol, 1, i32::from(c));
        }
    }
    if input.is_ascii_alpha_or_above() && !is_white_space(input.ch) {
        let mut prefix = [0u32; 3];
        let length = input.read_word(&mut prefix);
        let (kind, value) = keyword_lookup(&prefix, length);
        return Token::new(Tag::Keyword(kind), length, value);
    }
    if input.skip_white_space() {
        return Token::new(Tag::WhiteSpace, input.position() - pre_pos, 0);
    }
    if input.skip_parentheses() {
        return Token::new(Tag::Unknown, 0, 0);
    }
    input.next();
    Token::new(Tag::Unknown, 0, 0)
}

const KEYWORDS: &[(&[u8; 3], KeywordType, i32)] = &[
    (b"jan", KeywordType::MonthName, 1),
    (b"feb", KeywordType::MonthName, 2),
    (b"mar", KeywordType::MonthName, 3),
    (b"apr", KeywordType::MonthName, 4),
    (b"may", KeywordType::MonthName, 5),
    (b"jun", KeywordType::MonthName, 6),
    (b"jul", KeywordType::MonthName, 7),
    (b"aug", KeywordType::MonthName, 8),
    (b"sep", KeywordType::MonthName, 9),
    (b"oct", KeywordType::MonthName, 10),
    (b"nov", KeywordType::MonthName, 11),
    (b"dec", KeywordType::MonthName, 12),
    (b"am\0", KeywordType::AmPm, 0),
    (b"pm\0", KeywordType::AmPm, 12),
    (b"ut\0", KeywordType::TimeZoneName, 0),
    (b"utc", KeywordType::TimeZoneName, 0),
    (b"z\0\0", KeywordType::TimeZoneName, 0),
    (b"gmt", KeywordType::TimeZoneName, 0),
    (b"cdt", KeywordType::TimeZoneName, -5),
    (b"cst", KeywordType::TimeZoneName, -6),
    (b"edt", KeywordType::TimeZoneName, -4),
    (b"est", KeywordType::TimeZoneName, -5),
    (b"mdt", KeywordType::TimeZoneName, -6),
    (b"mst", KeywordType::TimeZoneName, -7),
    (b"pdt", KeywordType::TimeZoneName, -7),
    (b"pst", KeywordType::TimeZoneName, -8),
    (b"t\0\0", KeywordType::TimeSeparator, 0),
];

fn keyword_lookup(prefix: &[u32; 3], len: i32) -> (KeywordType, i32) {
    KEYWORDS
        .iter()
        .find(|(word, kind, _)| {
            word.iter().zip(prefix).all(|(&w, &p)| u32::from(w) == p)
                && (len <= 3 || *kind == KeywordType::MonthName)
        })
        .map_or((KeywordType::Invalid, 0), |&(_, kind, value)| (kind, value))
}

#[derive(Default)]
struct DayComposer {
    comp: [i32; 3],
    index: usize,
    named_month: Option<i32>,
    is_iso_date: bool,
}

impl DayComposer {
    fn is_empty(&self) -> bool {
        self.index == 0
    }

    fn add(&mut self, n: i32) -> bool {
        if self.index < 3 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn write(&mut self, out: &mut Output) -> bool {
        if self.index < 1 {
            return false;
        }
        while self.index < 3 {
            self.comp[self.index] = 1;
            self.index += 1;
        }
        let comp = self.comp;
        let (mut year, month, day);
        match self.named_month {
            None => {
                // `index` is always 3 here, so this is YMD unless the first
                // component could be a day.
                if self.is_iso_date || !is_day(comp[0]) {
                    (year, month, day) = (comp[0], comp[1], comp[2]);
                } else {
                    (month, day, year) = (comp[0], comp[1], comp[2]);
                }
            }
            Some(named) => {
                month = named;
                if !is_day(comp[0]) {
                    (year, day) = (comp[0], comp[1]);
                } else {
                    (day, year) = (comp[0], comp[1]);
                }
            }
        }
        if !self.is_iso_date {
            if (0..=49).contains(&year) {
                year += 2000;
            } else if (50..=99).contains(&year) {
                year += 1900;
            }
        }
        if !is_month(month) || !is_day(day) {
            return false;
        }
        out.year = f64::from(year);
        out.month = f64::from(month - 1);
        out.day = f64::from(day);
        true
    }
}

struct TimeComposer {
    comp: [i32; 4],
    index: usize,
    hour_offset: Option<i32>,
}

impl TimeComposer {
    fn new() -> Self {
        TimeComposer {
            comp: [0; 4],
            index: 0,
            hour_offset: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.index == 0
    }

    fn is_expecting(&self, n: i32) -> bool {
        (self.index == 1 && is_minute(n))
            || (self.index == 2 && is_second(n))
            || (self.index == 3 && is_millisecond(n))
    }

    fn add(&mut self, n: i32) -> bool {
        if self.index < 4 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn add_final(&mut self, n: i32) -> bool {
        if !self.add(n) {
            return false;
        }
        while self.index < 4 {
            self.comp[self.index] = 0;
            self.index += 1;
        }
        true
    }

    fn write(&mut self, out: &mut Output) -> bool {
        while self.index < 4 {
            self.comp[self.index] = 0;
            self.index += 1;
        }
        let [mut hour, minute, second, millisecond] = self.comp;
        if let Some(offset) = self.hour_offset {
            if !(0..=12).contains(&hour) {
                return false;
            }
            hour %= 12;
            hour += offset;
        }
        if (!is_hour(hour)
            || !is_minute(minute)
            || !is_second(second)
            || !is_millisecond(millisecond))
            && (hour != 24 || minute != 0 || second != 0 || millisecond != 0)
        {
            return false;
        }
        out.hour = f64::from(hour);
        out.minute = f64::from(minute);
        out.second = f64::from(second);
        out.millisecond = f64::from(millisecond);
        true
    }
}

struct TimeZoneComposer {
    sign: i32,
    hour: i32,
    minute: i32,
}

impl TimeZoneComposer {
    fn new() -> Self {
        TimeZoneComposer {
            sign: NONE,
            hour: NONE,
            minute: NONE,
        }
    }

    fn set(&mut self, offset_in_hours: i32) {
        self.sign = if offset_in_hours < 0 { -1 } else { 1 };
        self.hour = offset_in_hours * self.sign;
        self.minute = 0;
    }

    fn set_sign(&mut self, sign: i32) {
        self.sign = if sign < 0 { -1 } else { 1 };
    }

    fn is_expecting(&self, n: i32) -> bool {
        self.hour != NONE && self.minute == NONE && is_minute(n)
    }

    fn is_utc(&self) -> bool {
        self.hour == 0 && self.minute == 0
    }

    fn is_empty(&self) -> bool {
        self.hour == NONE
    }

    fn write(&mut self, out: &mut Output) -> bool {
        if self.sign == NONE {
            out.utc_offset = None;
            return true;
        }
        if self.hour == NONE {
            self.hour = 0;
        }
        if self.minute == NONE {
            self.minute = 0;
        }
        let total = (self.hour as u32)
            .wrapping_mul(3600)
            .wrapping_add((self.minute as u32).wrapping_mul(60));
        if total > SMI_MAX {
            return false;
        }
        let total = total as i32;
        out.utc_offset = Some(f64::from(if self.sign < 0 { -total } else { total }));
        true
    }
}

fn is_month(x: i32) -> bool {
    (1..=12).contains(&x)
}
fn is_day(x: i32) -> bool {
    (1..=31).contains(&x)
}
fn is_hour(x: i32) -> bool {
    (0..=23).contains(&x)
}
fn is_minute(x: i32) -> bool {
    (0..=59).contains(&x)
}
fn is_second(x: i32) -> bool {
    (0..=59).contains(&x)
}
fn is_millisecond(x: i32) -> bool {
    (0..=999).contains(&x)
}

fn read_milliseconds(token: Token) -> i32 {
    let mut number = token.value;
    let mut length = token.length;
    if length == 1 {
        number *= 100;
    } else if length == 2 {
        number *= 10;
    } else if length > 3 {
        length = length.min(MAX_SIGNIFICANT_DIGITS);
        let mut factor = 1;
        while length > 3 {
            factor *= 10;
            length -= 1;
        }
        number /= factor;
    }
    number
}

fn parse_components(units: &[u16]) -> Option<Output> {
    let mut scanner = Tokenizer::new(units);
    let mut day = DayComposer::default();
    let mut time = TimeComposer::new();
    let mut tz = TimeZoneComposer::new();

    let next_unhandled = parse_es5_date_time(&mut scanner, &mut day, &mut time, &mut tz);
    if next_unhandled.is_invalid() {
        return None;
    }
    let mut has_read_number = !day.is_empty();
    let mut token = next_unhandled;
    while !token.is_end_of_input() {
        if token.is_number() {
            has_read_number = true;
            let n = token.value;
            if scanner.skip_symbol(b':') {
                if scanner.skip_symbol(b':') {
                    if !time.is_empty() {
                        return None;
                    }
                    time.add(n);
                    time.add(0);
                } else {
                    if !time.add(n) {
                        return None;
                    }
                    if scanner.peek().is_symbol(b'.') {
                        scanner.next();
                    }
                }
            } else if scanner.skip_symbol(b'.') && time.is_expecting(n) {
                time.add(n);
                if !scanner.peek().is_number() {
                    return None;
                }
                let value = read_milliseconds(scanner.next());
                if value < 0 {
                    return None;
                }
                time.add_final(value);
            } else if tz.is_expecting(n) {
                tz.minute = n;
            } else if time.is_expecting(n) {
                time.add_final(n);
                let peek = scanner.peek();
                if !peek.is_end_of_input()
                    && !peek.is_white_space()
                    && !peek.is_keyword_z()
                    && !peek.is_ascii_sign()
                {
                    return None;
                }
            } else {
                if !day.add(n) {
                    return None;
                }
                scanner.skip_symbol(b'-');
            }
        } else if token.is_keyword() {
            let value = token.value;
            if token.is_keyword_type(KeywordType::AmPm) && !time.is_empty() {
                time.hour_offset = Some(value);
            } else if token.is_keyword_type(KeywordType::MonthName) {
                day.named_month = Some(value);
                scanner.skip_symbol(b'-');
            } else if token.is_keyword_type(KeywordType::TimeZoneName) && has_read_number {
                tz.set(value);
            } else {
                // Garbage words are illegal once a number has been read, and
                // must be separated from the first number.
                if has_read_number || scanner.peek().is_number() {
                    return None;
                }
            }
        } else if token.is_ascii_sign() && (tz.is_utc() || !time.is_empty()) {
            tz.set_sign(token.ascii_sign());
            let mut n = 0;
            let mut length = 0;
            if scanner.peek().is_number() {
                let next = scanner.next();
                length = next.length;
                n = next.value;
            }
            has_read_number = true;
            if scanner.peek().is_symbol(b':') {
                tz.hour = n;
                tz.minute = NONE;
            } else if length == 1 || length == 2 {
                tz.hour = n;
                tz.minute = 0;
            } else if length == 3 || length == 4 {
                tz.hour = n / 100;
                tz.minute = n % 100;
            } else {
                return None;
            }
        } else if (token.is_ascii_sign() || token.is_symbol(b')')) && has_read_number {
            return None;
        }
        token = scanner.next();
    }

    let mut out = Output {
        year: 0.0,
        month: 0.0,
        day: 0.0,
        hour: 0.0,
        minute: 0.0,
        second: 0.0,
        millisecond: 0.0,
        utc_offset: None,
    };
    (day.write(&mut out) && time.write(&mut out) && tz.write(&mut out)).then_some(out)
}

fn parse_es5_date_time(
    scanner: &mut Tokenizer,
    day: &mut DayComposer,
    time: &mut TimeComposer,
    tz: &mut TimeZoneComposer,
) -> Token {
    // Mandatory date: [('-'|'+')yy]yyyy[':'MM[':'DD]]
    if scanner.peek().is_ascii_sign() {
        let sign_token = scanner.next();
        if !scanner.peek().is_fixed_length_number(6) {
            return sign_token;
        }
        let sign = sign_token.ascii_sign();
        let year = scanner.next().value;
        if sign < 0 && year == 0 {
            return sign_token;
        }
        day.add(sign * year);
    } else if scanner.peek().is_fixed_length_number(4) {
        day.add(scanner.next().value);
    } else {
        return scanner.next();
    }
    if scanner.skip_symbol(b'-') {
        if !scanner.peek().is_fixed_length_number(2) || !is_month(scanner.peek().value) {
            return scanner.next();
        }
        day.add(scanner.next().value);
        if scanner.skip_symbol(b'-') {
            if !scanner.peek().is_fixed_length_number(2) || !is_day(scanner.peek().value) {
                return scanner.next();
            }
            day.add(scanner.next().value);
        }
    }
    // Optional time: 'T'HH':'mm[':'ss['.'sss]]Z
    if !scanner.peek().is_keyword_type(KeywordType::TimeSeparator) {
        if !scanner.peek().is_end_of_input() {
            return scanner.next();
        }
    } else {
        scanner.next();
        if !scanner.peek().is_fixed_length_number(2) || !(0..=24).contains(&scanner.peek().value) {
            return Token::invalid();
        }
        // Allow 24:00[:00[.000]], but no other time starting with 24.
        let hour_is_24 = scanner.peek().value == 24;
        time.add(scanner.next().value);
        if !scanner.skip_symbol(b':') {
            return Token::invalid();
        }
        if !scanner.peek().is_fixed_length_number(2)
            || !is_minute(scanner.peek().value)
            || (hour_is_24 && scanner.peek().value > 0)
        {
            return Token::invalid();
        }
        time.add(scanner.next().value);
        if scanner.skip_symbol(b':') {
            if !scanner.peek().is_fixed_length_number(2)
                || !is_second(scanner.peek().value)
                || (hour_is_24 && scanner.peek().value > 0)
            {
                return Token::invalid();
            }
            time.add(scanner.next().value);
            if scanner.skip_symbol(b'.') {
                if !scanner.peek().is_number() || (hour_is_24 && scanner.peek().value > 0) {
                    return Token::invalid();
                }
                time.add(read_milliseconds(scanner.next()));
            }
        }
        if scanner.peek().is_keyword_z() {
            scanner.next();
            tz.set(0);
        } else if scanner.peek().is_symbol(b'+') || scanner.peek().is_symbol(b'-') {
            tz.set_sign(if scanner.next().value == i32::from(b'+') {
                1
            } else {
                -1
            });
            if scanner.peek().is_fixed_length_number(4) {
                // hhmm extension syntax.
                let hourmin = scanner.next().value;
                let (hour, min) = (hourmin / 100, hourmin % 100);
                if !is_hour(hour) || !is_minute(min) {
                    return Token::invalid();
                }
                tz.hour = hour;
                tz.minute = min;
            } else {
                if !scanner.peek().is_fixed_length_number(2) || !is_hour(scanner.peek().value) {
                    return Token::invalid();
                }
                tz.hour = scanner.next().value;
                if !scanner.skip_symbol(b':') {
                    return Token::invalid();
                }
                if !scanner.peek().is_fixed_length_number(2) || !is_minute(scanner.peek().value) {
                    return Token::invalid();
                }
                tz.minute = scanner.next().value;
            }
        }
        if !scanner.peek().is_end_of_input() {
            return Token::invalid();
        }
    }
    // Date-only forms are UTC; date-time forms without an offset are local.
    if tz.is_empty() && time.is_empty() {
        tz.set(0);
    }
    day.is_iso_date = true;
    Token::new(Tag::EndOfInput, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(s: &str) -> Option<String> {
        to_iso_string(parse(s))
    }

    /// tests/js_date_corpus.json holds `new Date(input).toISOString()` from
    /// Node (TZ=UTC), generated by js/build_js_date_corpus.js.
    #[test]
    fn matches_v8_corpus() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/dates/js_date_corpus.json"
        );
        let corpus: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let failures: Vec<String> = corpus
            .iter()
            .filter_map(|case| {
                let input = case["input"].as_str().unwrap();
                let expected = case["expected"].as_str();
                let actual = iso(input);
                (actual.as_deref() != expected)
                    .then(|| format!("{input:?}: expected {expected:?}, got {actual:?}"))
            })
            .collect();
        assert!(
            failures.is_empty(),
            "{} of {} cases failed:\n{}",
            failures.len(),
            corpus.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn parses_like_v8() {
        let cases = [
            ("Jan 1 2020 10:00am", None),
            ("Jan 1 2020 10:00 am", Some("2020-01-01T10:00:00.000Z")),
            ("March 5", Some("2001-03-05T00:00:00.000Z")),
            ("12/25", Some("2001-12-25T00:00:00.000Z")),
            ("2020-01-01T10", None),
            ("5.3.2020", Some("2020-05-03T00:00:00.000Z")),
            ("Mar 5 2019 10:00:00.5", Some("2019-03-05T10:00:00.500Z")),
            ("20190305", None),
            ("Jan 5 GMT+2", Some("2001-01-04T22:00:00.000Z")),
            ("-000000-01-01", Some("2001-01-01T00:00:00.000Z")),
            ("junk 5/1/2020", Some("2001-06-05T00:00:00.000Z")),
            (
                "Jan 5 2020 10:00 +596523:00",
                Some("1951-12-18T07:00:00.000Z"),
            ),
            (
                "Jan 5 2020 10:00:00.0001234567890",
                Some("2020-01-05T10:00:00.123Z"),
            ),
            ("Jan 5 2020\u{2028}10:00", None),
            ("Jan\u{0}5 2020", None),
            ("2020-02-30", Some("2020-03-01T00:00:00.000Z")),
            ("Jan 5 2020 24:00", Some("2020-01-06T00:00:00.000Z")),
        ];
        for (input, expected) in cases {
            assert_eq!(iso(input).as_deref(), expected, "{input:?}");
        }
    }
}
