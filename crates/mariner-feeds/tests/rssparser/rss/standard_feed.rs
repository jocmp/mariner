//! `feed-test.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test.xml",
        title: Some("Android Authority"),
        site_url: Some("https://www.androidauthority.com"),
        description: Some("Android News, Reviews, How To"),
        entry_id: Some("https://www.androidauthority.com/?p=986823"),
        entry_title: Some("Japan’s plans for 5G include 10 billion 14-digit phone numbers"),
        entry_author: Some("Williams Pelegrin"),
        entry_url: Some(
            "https://www.androidauthority.com/japan-phone-numbers-14-digits-5g-986823/",
        ),
        categories: Some(&["News", "5G", "Japan"]),
        summary_prefix: Some(
            "Japan expects a huge surge of 5G-enabled devices to hit the market in the future",
        ),
        content_prefix: Some(
            "<p><a href=\"https://cdn57.androidauthority.net/wp-content/uploads/2019/02/Whats-",
        ),
        ..Case::DEFAULT
    }
    .check();
}
