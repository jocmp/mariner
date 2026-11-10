//! `src/extractors/generic/date-published/`.

use std::sync::LazyLock;

use dom_query::Document;
use regex::Regex;

use crate::cleaners::clean_date_published;
use crate::utils::dom::{extract_from_meta, extract_from_selectors};
use crate::utils::text::extract_from_url;

// Meta tag names that denote likely date published dates, from most
// distinct to least distinct.
const DATE_PUBLISHED_META_TAGS: &[&str] = &[
    "article:published_time",
    "displaydate",
    "dc.date",
    "dc.date.issued",
    "rbpubdate",
    "publish_date",
    "pub_date",
    "pagedate",
    "pubdate",
    "revision_date",
    "doc_date",
    "date_created",
    "content_create_date",
    "lastmodified",
    "created",
    "date",
];

const DATE_PUBLISHED_SELECTORS: &[&str] = &[
    ".hentry .dtstamp.published",
    ".hentry .published",
    ".hentry .dtstamp.updated",
    ".hentry .updated",
    ".single .published",
    ".meta .published",
    ".meta .postDate",
    ".entry-date",
    ".byline .date",
    ".postmetadata .date",
    ".article_datetime",
    ".date-header",
    ".story-date",
    ".dateStamp",
    "#story .datetime",
    ".dateline",
    ".pubdate",
];

// Expressions that find likely date published dates in the URL. The first
// capture group is always the date string.
static DATE_PUBLISHED_URL_RES: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    let abbrev_months = "(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)";
    [
        Regex::new(r"(?i)/(20\d{2}/\d{2}/\d{2})/").unwrap(),
        Regex::new(r"(?i)(20\d{2}-[01]\d-[0-3]\d)").unwrap(),
        Regex::new(&format!(r"(?i)/(20\d{{2}}/{abbrev_months}/[0-3]\d)/")).unwrap(),
    ]
});

/// Find the publication date as an ISO 8601 string.
pub fn extract(doc: &Document, url: &str, meta_cache: &[String]) -> Option<String> {
    // First, check to see if we have a matching meta tag that we can make
    // use of. Don't try cleaning tags from this string.
    if let Some(date) = extract_from_meta(doc, DATE_PUBLISHED_META_TAGS, meta_cache, false) {
        return clean_date_published(&date, None, None);
    }

    // Second, look through our selectors looking for potential dates.
    if let Some(date) = extract_from_selectors(doc, DATE_PUBLISHED_SELECTORS, 1, true) {
        return clean_date_published(&date, None, None);
    }

    // Lastly, look to see if a dately string exists in the URL.
    let regexes: Vec<&Regex> = DATE_PUBLISHED_URL_RES.iter().collect();
    extract_from_url(url, &regexes).and_then(|date| clean_date_published(&date, None, None))
}
