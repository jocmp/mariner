//! `feed-item-author.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-item-author.xml",
        title: Some("Planet KDE | English"),
        site_url: Some("https://planet.kde.org/"),
        description: Some("Planet KDE | English"),
        entry_id: Some("https://blogs.kde.org/2025/03/24/this-week-in-kde-apps/"),
        entry_title: Some("This Week in KDE Apps"),
        entry_author: Some("This Week in KDE Apps"),
        entry_url: Some("https://blogs.kde.org/2025/03/24/this-week-in-kde-apps/"),
        summary_prefix: Some(
            "<h4>Stability improvements in KDE PIM-land, and polls in NeoChat</h4>",
        ),
        ..Case::DEFAULT
    }
    .check();
}
