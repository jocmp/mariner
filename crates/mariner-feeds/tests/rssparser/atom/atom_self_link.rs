//! `atom-self-link-example.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-self-link-example.xml",
        title: Some("Simon Willison's Weblog"),
        site_url: Some("http://simonwillison.net/"),
        entry_id: Some(
            "http://simonwillison.net/2023/Jul/29/a-steering-council-notice-about-pep-703/#atom-everything",
        ),
        entry_title: Some(
            "A Steering Council notice about PEP 703 (Making the Global Interpreter Lock Optional\n            in CPython)",
        ),
        entry_url: Some(
            "http://simonwillison.net/2023/Jul/29/a-steering-council-notice-about-pep-703/#atom-everything",
        ),
        summary_prefix: Some("<p><a"),
        ..Case::DEFAULT
    }
    .check();
}
