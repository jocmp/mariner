//! `atom-feed-content.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-feed-content.xml",
        title: Some("GrapheneOS changelog"),
        site_url: Some("https://grapheneos.org/LICENSE.txt"),
        entry_id: Some("https://grapheneos.org/releases#2025041100"),
        entry_title: Some("2025041100"),
        entry_url: Some("https://grapheneos.org/releases#2025041100"),
        ..Case::DEFAULT
    }
    .check();
}
