//! `atom-feed-created-updated.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-feed-created-updated.xml",
        title: Some("tonsky.me"),
        site_url: Some("https://tonsky.me/"),
        entry_id: Some("https://tonsky.me/blog/unicode/"),
        entry_title: Some(
            "The Absolute Minimum Every Software Developer Must Know About Unicode in 2023 (Still No Excuses!)",
        ),
        entry_author: Some("Nikita Prokopov"),
        entry_url: Some("https://tonsky.me/blog/unicode/"),
        summary_prefix: Some("Modern extension to classic 2003 article by Joel Spolsky"),
        ..Case::DEFAULT
    }
    .check();
}
