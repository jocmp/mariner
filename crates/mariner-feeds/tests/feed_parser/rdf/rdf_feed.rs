//! `feed-rdf-test.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-rdf-test.xml",
        title: Some("Slashdot"),
        site_url: Some("https://slashdot.org/"),
        description: Some("News for nerds, stuff that matters"),
        entry_title: Some(
            "Google Play Will No Longer Pay To Discover Vulnerabilities In Popular Android Apps",
        ),
        entry_author: Some("BeauHD"),
        entry_url: Some(
            "https://tech.slashdot.org/story/24/08/22/2042250/google-play-will-no-longer-pay-to-discover-vulnerabilities-in-popular-android-apps?utm_source=rss1.0mainlinkanon&utm_medium=feed",
        ),
        summary_prefix: Some(
            "Android Authority's Mishaal Rahman reports: Security vulnerabilities are lurking",
        ),
        ..Case::DEFAULT
    }
    .check();
}
