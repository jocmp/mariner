//! `src/extractors/generic/title/`.

use dom_query::Document;

use crate::cleaners::clean_title;
use crate::utils::dom::{extract_from_meta, extract_from_selectors};

// Meta tag names that denote likely article titles, from most distinct to
// least distinct.
const STRONG_TITLE_META_TAGS: &[&str] = &[
    "tweetmeme-title",
    "dc.title",
    "rbtitle",
    "headline",
    "title",
];

// og:title is weak because it typically contains context that we don't
// like, for example the source site's name.
const WEAK_TITLE_META_TAGS: &[&str] = &["og:title"];

const STRONG_TITLE_SELECTORS: &[&str] = &[
    ".hentry .entry-title",
    "h1#articleHeader",
    "h1.articleHeader",
    "h1.article",
    ".instapaper_title",
    "#meebo-title",
];

const WEAK_TITLE_SELECTORS: &[&str] = &[
    "article h1",
    "#entry-title",
    ".entry-title",
    "#entryTitle",
    "#entrytitle",
    ".entryTitle",
    ".entrytitle",
    "#articleTitle",
    ".articleTitle",
    "post post-title",
    "h1.title",
    "h2.article",
    "h1",
    "html head title",
    "title",
];

/// Find the article title, or `""` when nothing matches.
pub fn extract(doc: &Document, url: &str, meta_cache: &[String]) -> String {
    let title = extract_from_meta(doc, STRONG_TITLE_META_TAGS, meta_cache, true)
        .or_else(|| extract_from_selectors(doc, STRONG_TITLE_SELECTORS, 1, true))
        .or_else(|| extract_from_meta(doc, WEAK_TITLE_META_TAGS, meta_cache, true))
        .or_else(|| extract_from_selectors(doc, WEAK_TITLE_SELECTORS, 1, true));

    title.map_or_else(String::new, |title| clean_title(&title, url, doc))
}
