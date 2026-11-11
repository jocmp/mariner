//! `feed-npr-world.json`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-npr-world.json",
        title: Some("World"),
        site_url: Some(
            "https://www.npr.org/sections/world/?utm_medium=JSONFeed&utm_campaign=world",
        ),
        description: Some(
            "NPR world news, international art and culture, world business and financial markets, world economy, and global trends in health, science and technology. Subscribe to the World Story of the Day podcast and RSS feed.",
        ),
        entry_title: Some("Eggs and Bananas: Life after a Russian prison"),
        entry_url: Some(
            "https://www.npr.org/2024/08/26/1198913145/eggs-and-bananas-life-after-a-russian-prison?utm_medium=JSONFeed&utm_campaign=world",
        ),
        summary_prefix: Some(
            "It's been more than three weeks since the U.S. and Russia completed the largest ",
        ),
        content_prefix: Some(
            "<p>It's been more than three weeks since the U.S. and Russia completed the large",
        ),
        ..Case::DEFAULT
    }
    .check();
}
