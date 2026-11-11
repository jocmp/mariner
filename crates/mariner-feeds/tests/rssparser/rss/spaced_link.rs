//! `feed-spaced-link.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-spaced-link.xml",
        title: Some("Sixth Tone RSS"),
        site_url: Some("https://www.sixthtone.com"),
        description: Some("Sixth Tone RSS"),
        entry_title: Some("Feathered Dinosaur From China Points to Another Path to Flight"),
        entry_author: Some("Fan Yiying"),
        entry_url: Some(
            "https://www.sixthtone.com/news/1019094/Feathered Dinosaur From China Points to Another Path to Flight",
        ),
        categories: Some(&["NEWS"]),
        summary_prefix: Some(
            "<a href=\"https://www.sixthtone.com/news/1019094/Feathered Dinosaur From China Po",
        ),
        ..Case::DEFAULT
    }
    .check();
}
