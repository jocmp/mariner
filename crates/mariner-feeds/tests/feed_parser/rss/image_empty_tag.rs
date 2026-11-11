//! `feed-test-image-empty-tag.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-image-empty-tag.xml",
        title: Some("Hacker Noon"),
        site_url: Some("https://hackernoon.com"),
        description: Some("How hackers start their afternoons."),
        entry_id: Some("https://hackernoon.com/miscellaneous-directions?source=rss"),
        entry_title: Some("MISCELLANEOUS DIRECTIONS."),
        entry_author: Some("Catharine Esther Beecher"),
        entry_url: Some("https://hackernoon.com/miscellaneous-directions?source=rss"),
        categories: Some(&["domestic-manuals"]),
        summary_prefix: Some(
            "Every woman should know how to direct in regard to the proper care of domestic a",
        ),
        ..Case::DEFAULT
    }
    .check();
}
