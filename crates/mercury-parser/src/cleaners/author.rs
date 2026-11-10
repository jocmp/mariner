//! `src/cleaners/author.js`.

use std::sync::LazyLock;

use regex::Regex;

use crate::js_ws;
use crate::utils::js;
use crate::utils::text::normalize_spaces;

static CLEAN_AUTHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "(?i)^[",
        js_ws!(),
        "]*(posted |written )?by[",
        js_ws!(),
        "]*:?[",
        js_ws!(),
        r"]*([^\n\r\x{2028}\x{2029}]*)"
    ))
    .unwrap()
});

/// Take an author string (like `'By David Smith '`) and clean it to just
/// the name(s): `'David Smith'`.
pub fn clean_author(author: &str) -> String {
    let author = CLEAN_AUTHOR_RE.replace(author, "$2");
    normalize_spaces(js::trim(&author))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_the_by_prefix() {
        assert_eq!(clean_author("By Bob Dylan"), "Bob Dylan");
        assert_eq!(
            clean_author("Posted by: Rhiannon   Giddens "),
            "Rhiannon Giddens"
        );
        assert_eq!(clean_author("written by\nBob"), "Bob");
    }
}
