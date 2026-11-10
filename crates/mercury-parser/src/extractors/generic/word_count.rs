//! `src/extractors/generic/word-count/`.

use std::sync::LazyLock;

use regex::Regex;

use crate::js_ws;
use crate::utils::dom::{parse_document, select};
use crate::utils::js;
use crate::utils::text::normalize_spaces;

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("<[^>]*>").unwrap());
static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]")).unwrap());
static WHITESPACE_RUN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[", js_ws!(), "]+")).unwrap());

/// Count the words in the content HTML.
pub fn extract(content: &str) -> usize {
    let count = word_count(content);
    if count == 1 {
        word_count_alt(content)
    } else {
        count
    }
}

fn word_count(content: &str) -> usize {
    let doc = parse_document(content);
    let text = normalize_spaces(&select(&doc, "div").first().text());
    WHITESPACE_RE.split(&text).count()
}

fn word_count_alt(content: &str) -> usize {
    let content = TAG_RE.replace_all(content, " ");
    let content = WHITESPACE_RUN_RE.replace_all(&content, " ");
    js::trim(&content).split(' ').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words() {
        assert_eq!(
            extract("<div><p>One two</p><p>three four five</p></div>"),
            4
        );
        assert_eq!(extract("<div><p>One two </p> <p>three</p></div>"), 3);
        assert_eq!(extract("<span>one two</span>"), 2);
    }
}
