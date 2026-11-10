//! `src/extractors/generic/author/`.

use std::sync::LazyLock;

use dom_query::Document;
use regex::Regex;

use crate::cleaners::clean_author;
use crate::js_ws;
use crate::utils::dom::{extract_from_meta, extract_from_selectors, select};
use crate::utils::js;

// Meta tag names that denote likely article authors, from most distinct to
// least distinct. "author" is too often the -developer- of the page, so it
// is not added here.
const AUTHOR_META_TAGS: &[&str] = &[
    "byl",
    "clmst",
    "dc.author",
    "dcsext.author",
    "dc.creator",
    "rbauthors",
    "authors",
];

const AUTHOR_MAX_LENGTH: usize = 300;

const AUTHOR_SELECTORS: &[&str] = &[
    ".entry .entry-author",
    ".author.vcard .fn",
    ".author .vcard .fn",
    ".byline.vcard .fn",
    ".byline .vcard .fn",
    ".byline .by .author",
    ".byline .by",
    ".byline .author",
    ".post-author.vcard",
    ".post-author .vcard",
    "a[rel=author]",
    "#by_author",
    ".by_author",
    "#entryAuthor",
    ".entryAuthor",
    ".byline a[href*=author]",
    "#author .authorname",
    ".author .authorname",
    "#author",
    ".author",
    ".articleauthor",
    ".ArticleAuthor",
    ".byline",
];

static BYLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(concat!("(?i)^[\n", js_ws!(), "]*By")).unwrap());
const BYLINE_SELECTORS: &[&str] = &["#byline", ".byline"];

/// Find the article's author.
pub fn extract(doc: &Document, meta_cache: &[String]) -> Option<String> {
    // First, check to see if we have a matching meta tag that we can make
    // use of.
    if let Some(author) = extract_from_meta(doc, AUTHOR_META_TAGS, meta_cache, true)
        && js::len(&author) < AUTHOR_MAX_LENGTH
    {
        return Some(clean_author(&author));
    }

    // Second, look through our selectors looking for potential authors.
    if let Some(author) = extract_from_selectors(doc, AUTHOR_SELECTORS, 2, true)
        && js::len(&author) < AUTHOR_MAX_LENGTH
    {
        return Some(clean_author(&author));
    }

    // Last, use our looser regular-expression based selectors for
    // potential authors.
    for selector in BYLINE_SELECTORS {
        let node = select(doc, selector);
        if node.length() == 1 {
            let text = node.text();
            if BYLINE_RE.is_match(&text) {
                return Some(clean_author(&text));
            }
        }
    }

    None
}
