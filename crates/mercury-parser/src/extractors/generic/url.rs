//! `src/extractors/generic/url/`.

use dom_query::Document;

use crate::utils::dom::{extract_from_meta, select};
use crate::utils::url::parse_url;

const CANONICAL_META_SELECTORS: &[&str] = &["og:url"];

/// The canonical URL for the page, and its hostname.
pub fn extract(doc: &Document, url: &str, meta_cache: &[String]) -> (String, Option<String>) {
    let canonical = select(doc, "link[rel=canonical]");
    if !canonical.is_empty()
        && let Some(href) = canonical.attr("href").filter(|href| !href.is_empty())
    {
        return result(&href);
    }

    if let Some(meta_url) = extract_from_meta(doc, CANONICAL_META_SELECTORS, meta_cache, true) {
        return result(&meta_url);
    }

    result(url)
}

fn result(url: &str) -> (String, Option<String>) {
    let domain = parse_url(url).and_then(|parsed| parsed.host_str().map(ToString::to_string));
    (url.to_string(), domain)
}
