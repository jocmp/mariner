//! `atom-test-example.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-test-example.xml",
        title: Some("Example Feed"),
        site_url: Some("http://example.org/"),
        description: Some("A subtitle."),
        entry_id: Some("urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a"),
        entry_title: Some("Atom-Powered Robots Run Amok"),
        entry_author: Some("John Doe"),
        entry_url: Some("http://example.org/2003/12/13/atom03.html"),
        summary_prefix: Some("Some text."),
        ..Case::DEFAULT
    }
    .check();
}
