//! `src/cleaners/title.js`.

use std::sync::LazyLock;

use dom_query::Document;
use regex::Regex;

use super::resolve_split_title;
use crate::utils::dom::{select, strip_tags};
use crate::utils::js;
use crate::utils::text::normalize_spaces;

/// Separating characters on a title that usually denote breadcrumbs or
/// something similar.
pub static TITLE_SPLITTERS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(: | - | \| )").unwrap());

/// Clean a title: drop breadcrumb or site-name segments, fall back to the
/// lone `<h1>` for absurdly long titles, and strip tags.
pub fn clean_title(title: &str, url: &str, doc: &Document) -> String {
    let mut title = title.to_string();

    // If the title has |, :, or - in it, see if we can clean it up.
    if TITLE_SPLITTERS_RE.is_match(&title) {
        title = resolve_split_title(&title, url);
    }

    // Final sanity check that we didn't get a crazy title. If we did, use
    // the h1 from the document if there's exactly one.
    if js::len(&title) > 150 {
        let h1 = select(doc, "h1");
        if h1.length() == 1 {
            title = h1.text().to_string();
        }
    }

    // Strip any HTML tags in the title text.
    normalize_spaces(js::trim(&strip_tags(&title)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::dom::parse_document;

    #[test]
    fn strips_tags_and_spaces() {
        let doc = parse_document("<p></p>");
        assert_eq!(
            clean_title("  A <b>bold</b>   title ", "", &doc),
            "A bold title"
        );
    }

    #[test]
    fn falls_back_to_h1_for_long_titles() {
        let doc = parse_document("<h1>Short Title</h1>");
        assert_eq!(clean_title(&"x".repeat(151), "", &doc), "Short Title");
    }
}
