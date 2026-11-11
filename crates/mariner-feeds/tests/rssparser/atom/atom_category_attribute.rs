//! `feed-test-atom-category-attribute.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-atom-category-attribute.xml",
        title: Some("Slashdot"),
        site_url: Some("http://pubsubhubbub.appspot.com/"),
        description: Some("News for nerds, stuff that matters"),
        entry_id: Some(
            "https://yro.slashdot.org/story/23/05/28/2213240/automakers-ask-judge-to-block-pending-enforcement-of-massachusetts-right-to-repair-law?utm_source=atom1.0mainlinkanon&utm_medium=feed",
        ),
        entry_title: Some(
            "Automakers Ask Judge to Block Pending Enforcement of Massachusetts' Right-to-Repair Law",
        ),
        entry_author: Some("EditorDavid"),
        entry_url: Some(
            "https://yro.slashdot.org/story/23/05/28/2213240/automakers-ask-judge-to-block-pending-enforcement-of-massachusetts-right-to-repair-law?utm_source=atom1.0mainlinkanon&utm_medium=feed",
        ),
        categories: Some(&["government"]),
        summary_prefix: Some(
            "\"Beginning next Thursday, Massachusetts Attorney General Andrea Joy Campbell pla",
        ),
        ..Case::DEFAULT
    }
    .check();
}
