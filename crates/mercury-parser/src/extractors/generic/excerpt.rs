//! `src/extractors/generic/excerpt/`.

use std::sync::LazyLock;

use dom_query::Document;
use regex::Regex;

use crate::js_ws;
use crate::utils::dom::{extract_from_meta, fragment_nodes, parse_fragment, strip_tags, text};
use crate::utils::ellipsize::ellipsize;
use crate::utils::js;

const EXCERPT_META_SELECTORS: &[&str] = &["og:description", "twitter:description"];
const MAX_LENGTH: usize = 200;

static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("[\n", js_ws!(), "]+")).unwrap());

/// A short summary: the page's description meta tag, or the start of the
/// extracted content.
pub fn extract(doc: &Document, content: Option<&str>, meta_cache: &[String]) -> String {
    if let Some(excerpt) = extract_from_meta(doc, EXCERPT_META_SELECTORS, meta_cache, true) {
        return clean(&strip_tags(&excerpt), MAX_LENGTH);
    }

    // Fall back to excerpting from the extracted content.
    let short_content = js::slice_to(content.unwrap_or_default(), MAX_LENGTH * 5);
    let fragment = parse_fragment(short_content);
    clean(&text(&fragment_nodes(&fragment)), MAX_LENGTH)
}

/// Collapse whitespace and truncate to `max_length`.
pub fn clean(content: &str, max_length: usize) -> String {
    let content = WHITESPACE_RE.replace_all(content, " ");
    ellipsize(js::trim(&content), max_length, "&hellip;")
}
