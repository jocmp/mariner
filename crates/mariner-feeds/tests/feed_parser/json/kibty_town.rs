//! `feed-kibty-town.json`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-kibty-town.json",
        title: Some("xyzeva's blog"),
        site_url: Some("https://kibty.town/"),
        description: Some("random thoughts and other stuff"),
        entry_title: Some("gaining access to anyones browser without them even visiting a website"),
        entry_url: Some("https://kibty.town/blog/arc/"),
        content_prefix: Some(
            "<p>we start at the homepage of arc. where i first landed when i first heard of i",
        ),
        ..Case::DEFAULT
    }
    .check();
}
