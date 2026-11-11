//! `feed-test-hindi-channel-image.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-hindi-channel-image.xml",
        title: Some("Latest News चीन News18 हिंदी"),
        site_url: Some("https://hindi.news18.com/rss/khabar/world/china.xml"),
        description: Some("Latest news from चीन Section"),
        entry_id: Some(
            "https://hindi.news18.com/news/world/who-experts-in-wuhan-probing-coronavirus-origins-meet-chinese-scientists-nodtg-3437941.html",
        ),
        entry_title: Some("कैसे फैला कोरोना वायरस, WHO की टीम करेगी चीनी अधिकारियों के साथ बैठक"),
        entry_url: Some(
            "https://hindi.news18.com/news/world/who-experts-in-wuhan-probing-coronavirus-origins-meet-chinese-scientists-nodtg-3437941.html",
        ),
        categories: Some(&[]),
        summary_prefix: Some(
            "<img src='https://images.news18.com/ibnkhabar/uploads/2021/01/who-team-1.jpg' he",
        ),
        ..Case::DEFAULT
    }
    .check();
}
