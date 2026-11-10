//! Text utilities, from `src/utils/text/`.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

use crate::js_ws;
use crate::utils::js;

// An expression that looks to try to find the page digit within a URL, if
// it exists.
// Matches:
//  page=1
//  pg=1
//  p=1
//  paging=12
//  pag=7
//  pagination/1
//  paging/88
//  pa/83
//  p/11
//
// Does not match:
//  pg=102
//  page:2
pub static PAGE_IN_HREF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(page|paging|(p(a|g|ag)?(e|enum|ewanted|ing|ination)))?(=|/)([0-9]{1,3})")
        .unwrap()
});

pub static HAS_ALPHA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[a-z]").unwrap());
pub static IS_ALPHA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z]+$").unwrap());
pub static IS_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]+$").unwrap());

static ENCODING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"charset=([A-Za-z0-9_-]+)(?-u:\b)").unwrap());
pub const DEFAULT_ENCODING: &str = "utf-8";

static WHITESPACE_RUN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]{2,}")).unwrap());
static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]+")).unwrap());
static OPEN_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(pre|code|textarea)[^>]*>").unwrap());
static CLOSE_PRE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</pre>").unwrap());
static CLOSE_CODE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</code>").unwrap());
static CLOSE_TEXTAREA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</textarea>").unwrap());
static SENTENCE_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\n\r\x{2028}\x{2029}]( |$)").unwrap());
static TRAILING_SLASH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/$").unwrap());

/// Collapse runs of whitespace, except inside `<pre>`, `<code>`, and
/// `<textarea>` blocks, and trim the result.
pub fn normalize_spaces(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut search_from = 0;

    while let Some(open) = OPEN_BLOCK_RE.captures_at(text, search_from) {
        let whole = open.get(0).unwrap();
        let close_re = match open[1].to_ascii_lowercase().as_str() {
            "pre" => &CLOSE_PRE_RE,
            "code" => &CLOSE_CODE_RE,
            _ => &CLOSE_TEXTAREA_RE,
        };

        // Unclosed block: nothing further to preserve, so collapse the rest.
        let Some(close) = close_re.find_at(text, whole.end()) else {
            break;
        };

        result.push_str(&WHITESPACE_RUN_RE.replace_all(&text[cursor..whole.start()], " "));
        result.push_str(&text[whole.start()..close.end()]);
        cursor = close.end();
        search_from = close.end();
    }

    result.push_str(&WHITESPACE_RUN_RE.replace_all(&text[cursor..], " "));
    js::trim(&result).to_string()
}

/// Return the first capture group of the first expression that matches
/// `url`.
pub fn extract_from_url(url: &str, regexes: &[&Regex]) -> Option<String> {
    regexes
        .iter()
        .find_map(|re| re.captures(url))
        .map(|captures| captures[1].to_string())
}

/// The page number in `url`, if it looks paginated and the number is
/// below 100.
pub fn page_num_from_url(url: &str) -> Option<i64> {
    let captures = PAGE_IN_HREF_RE.captures(url)?;
    let page_num = js::parse_int(&captures[6])?;

    (page_num < 100).then_some(page_num)
}

/// Drop the fragment and any trailing slash.
pub fn remove_anchor(url: &str) -> String {
    let without_hash = url.split('#').next().unwrap_or_default();
    TRAILING_SLASH_RE.replace(without_hash, "").into_owned()
}

/// Take a URL and return the article base of said URL: the URL with any
/// pagination segments removed. Useful for comparing to other links that
/// might have pagination data within them.
pub fn article_base_url(url: &str, parsed: Option<&Url>) -> String {
    let owned;
    let parsed = match parsed {
        Some(parsed) => parsed,
        None => match Url::parse(url) {
            Ok(url) => {
                owned = url;
                &owned
            }
            Err(_) => return url.to_string(),
        },
    };

    let protocol = format!("{}:", parsed.scheme());
    let host = match (parsed.host_str(), parsed.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => String::new(),
    };
    let search = match parsed.query() {
        Some(query) if !query.is_empty() => format!("?{query}"),
        _ => String::new(),
    };
    let path = format!("{}{}", parsed.path(), search);

    let mut first_segment_has_letters = false;
    let mut cleaned_segments = Vec::new();

    for (index, raw_segment) in path.split('/').rev().enumerate() {
        let mut segment = raw_segment.to_string();

        // Split off and save anything that looks like a file type.
        if segment.contains('.') {
            let mut parts = segment.split('.');
            let possible_segment = parts.next().unwrap_or_default().to_string();
            let file_ext = parts.next().unwrap_or_default();
            if IS_ALPHA_RE.is_match(file_ext) {
                segment = possible_segment;
            }
        }

        // If our first or second segment has anything looking like a page
        // number, remove it.
        if index < 2 && PAGE_IN_HREF_RE.is_match(&segment) {
            segment = PAGE_IN_HREF_RE.replace(&segment, "").into_owned();
        }

        // The first segment is actually the last bit of the URL; this helps
        // determine if we're on a URL segment that looks like "/2/".
        if index == 0 {
            first_segment_has_letters = HAS_ALPHA_RE.is_match(&segment);
        }

        if is_good_segment(&segment, index, first_segment_has_letters) {
            cleaned_segments.push(segment);
        }
    }

    cleaned_segments.reverse();
    format!("{protocol}//{host}{}", cleaned_segments.join("/"))
}

/// Whether `text` appears to contain the end of a sentence.
pub fn has_sentence_end(text: &str) -> bool {
    SENTENCE_END_RE.is_match(text)
}

/// The first `words` whitespace-separated words of `content`.
pub fn excerpt_content(content: &str, words: usize) -> String {
    WHITESPACE_RE
        .split(js::trim(content))
        .take(words)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The encoding named by a `Content-Type` header or `<meta>` value, falling
/// back to UTF-8 when it's missing or unknown.
pub fn get_encoding(value: Option<&str>) -> String {
    let Some(value) = value else {
        return DEFAULT_ENCODING.to_string();
    };
    let label = ENCODING_RE
        .captures(value)
        .map_or(value, |captures| captures.get(1).unwrap().as_str());

    match encoding_rs::Encoding::for_label(label.trim().as_bytes()) {
        Some(_) => label.to_ascii_lowercase(),
        None => DEFAULT_ENCODING.to_string(),
    }
}

fn is_good_segment(segment: &str, index: usize, first_segment_has_letters: bool) -> bool {
    // If this is the first url_segment and it's just "index", remove it.
    if index == 0 && segment.eq_ignore_ascii_case("index") {
        return false;
    }

    // If our first or second url_segment is smaller than 3 characters, and
    // the first url_segment had no alphas, remove it.
    !(index < 2 && js::len(segment) < 3 && !first_segment_has_letters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_spaces_outside_pre_blocks() {
        assert_eq!(normalize_spaces("  a   b \n\n c "), "a b c");
        assert_eq!(
            normalize_spaces("<p>a   b</p><pre>  x   y  </pre> c   d"),
            "<p>a b</p><pre>  x   y  </pre> c d"
        );
        assert_eq!(normalize_spaces("<pre>  unclosed   x"), "<pre> unclosed x");
    }

    #[test]
    fn finds_page_numbers() {
        assert_eq!(page_num_from_url("http://example.com/foo?page=2"), Some(2));
        assert_eq!(
            page_num_from_url("http://example.com/foo/pagination/13"),
            Some(13)
        );
        assert_eq!(page_num_from_url("http://example.com/foo?pg=102"), None);
        assert_eq!(page_num_from_url("http://example.com/foo"), None);
    }

    #[test]
    fn removes_anchors() {
        assert_eq!(
            remove_anchor("http://example.com/foo/#bar"),
            "http://example.com/foo"
        );
    }

    #[test]
    fn finds_article_base_urls() {
        assert_eq!(
            article_base_url("http://example.com/foo/bar/wow-cool/page=10", None),
            "http://example.com/foo/bar/wow-cool"
        );
        assert_eq!(
            article_base_url("http://example.com/foo/bar/wow-cool/2", None),
            "http://example.com/foo/bar/wow-cool"
        );
        assert_eq!(
            article_base_url("http://example.com/foo/bar/wow-cool/index.html", None),
            "http://example.com/foo/bar/wow-cool"
        );
    }

    #[test]
    fn detects_sentence_ends() {
        assert!(has_sentence_end("This is a sentence."));
        assert!(has_sentence_end("Two words"));
        assert!(!has_sentence_end(""));
    }

    #[test]
    fn excerpts_content() {
        assert_eq!(excerpt_content(" one two  three four ", 2), "one two");
    }

    #[test]
    fn gets_encodings() {
        assert_eq!(
            get_encoding(Some("text/html; charset=iso-8859-15")),
            "iso-8859-15"
        );
        assert_eq!(get_encoding(Some("text/html; charset=bogus")), "utf-8");
        assert_eq!(get_encoding(Some("Shift_JIS")), "shift_jis");
        assert_eq!(get_encoding(None), "utf-8");
    }
}
