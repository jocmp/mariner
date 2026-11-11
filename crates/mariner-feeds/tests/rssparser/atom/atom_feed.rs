//! `feed-atom-test.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-atom-test.xml",
        title: Some("The Verge -  All Posts"),
        site_url: Some("https://www.theverge.com/"),
        entry_id: Some(
            "https://www.theverge.com/2023/5/26/23739273/google-sonos-smart-speaker-patent-lawsuit-ruling",
        ),
        entry_title: Some("Sonos wins $32.5 million patent infringement victory over Google"),
        entry_author: Some("Chris Welch"),
        entry_url: Some(
            "https://www.theverge.com/2023/5/26/23739273/google-sonos-smart-speaker-patent-lawsuit-ruling",
        ),
        content_prefix: Some("<figure>"),
        ..Case::DEFAULT
    }
    .check();
}
