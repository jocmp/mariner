//! `src/cleaners/dek.js`.

use std::sync::LazyLock;

use regex::Regex;

use crate::utils::dom::strip_tags;
use crate::utils::js;
use crate::utils::text::{excerpt_content, normalize_spaces};

static TEXT_LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?i)http(s)?://").unwrap());

/// Take a dek HTML fragment and return the cleaned version of it, or
/// `None` if the dek wasn't good enough.
pub fn clean_dek(dek: &str, excerpt: Option<&str>) -> Option<String> {
    // Sanity check that we didn't get too short or long of a dek.
    let length = js::len(dek);
    if !(5..=1000).contains(&length) {
        return None;
    }

    // Check that the dek isn't the same as the excerpt.
    if let Some(excerpt) = excerpt.filter(|excerpt| !excerpt.is_empty())
        && excerpt_content(excerpt, 10) == excerpt_content(dek, 10)
    {
        return None;
    }

    // Plain text links shouldn't exist in the dek. If we have some, it's
    // not a good dek - bail.
    let dek_text = strip_tags(dek);
    if TEXT_LINK_RE.is_match(&dek_text) {
        return None;
    }

    Some(normalize_spaces(js::trim(&dek_text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_deks() {
        assert_eq!(clean_dek("Hi", None), None);
        assert_eq!(clean_dek("See https://example.com", None), None);
        assert_eq!(clean_dek("Same words here", Some("Same words here")), None);
    }

    #[test]
    fn cleans_good_deks() {
        assert_eq!(
            clean_dek("This is a <em>good</em>   dek.", None),
            Some("This is a good dek.".to_string())
        );
    }
}
