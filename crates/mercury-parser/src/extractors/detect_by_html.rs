//! `src/extractors/detect-by-html.js`: pick an extractor from markup, for
//! platforms hosted on arbitrary domains.

use dom_query::Document;

use super::CustomExtractor;
use super::custom::{blogspot_com, medium_com};
use crate::utils::dom::select;

/// The extractor for a Medium or Blogger page, if the markup says it is one.
pub fn detect_by_html(doc: &Document) -> Option<CustomExtractor> {
    if !select(doc, r#"meta[name="al:ios:app_name"][value="Medium"]"#).is_empty() {
        return Some(medium_com::extractor());
    }
    if !select(doc, r#"meta[name="generator"][value="blogger"]"#).is_empty() {
        return Some(blogspot_com::extractor());
    }
    None
}
