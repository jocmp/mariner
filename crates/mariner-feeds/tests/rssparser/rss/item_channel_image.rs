//! `feed-item-channel-image.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-item-channel-image.xml",
        title: Some("www.espn.com - TOP"),
        site_url: Some("https://www.espn.com"),
        description: Some("Latest TOP news from www.espn.com"),
        entry_id: Some("31393791"),
        entry_title: Some("Inside the mysterious world of missing sports memorabilia"),
        entry_url: Some(
            "https://www.espn.com/mlb/story/_/id/31393791/inside-mysterious-world-missing-sports-memorabilia",
        ),
        summary_prefix: Some(
            "Some of the most treasured pieces of sports memorabilia are missing, can't be au",
        ),
        ..Case::DEFAULT
    }
    .check();
}
