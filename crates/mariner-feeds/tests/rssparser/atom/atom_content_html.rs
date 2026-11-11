//! `feed-test-atom-content-html.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-atom-content-html.xml",
        title: Some("Jake Wharton"),
        site_url: Some("https://jakewharton.com/"),
        entry_id: Some("https://jakewharton.com/the-state-of-managing-state-with-compose"),
        entry_title: Some("The state of managing state (with Compose)"),
        entry_url: Some("https://code.cash.app/the-state-of-managing-state-with-compose"),
        content_prefix: Some(
            "This post was published externally on Cash App Code Blog. Read it at https://cod",
        ),
        ..Case::DEFAULT
    }
    .check();
}
