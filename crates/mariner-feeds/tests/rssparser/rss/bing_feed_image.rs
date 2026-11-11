//! `feed-bing-image.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-bing-image.xml",
        title: Some("madrid - BingNews"),
        site_url: Some("https://www.bing.com:443/news/search?q=madrid&format=rss"),
        description: Some("Search results"),
        entry_title: Some(
            "Real Madrid, Barcelona and Juventus 'threaten clubs withdrawing from Super League\n                with legal action'",
        ),
        entry_url: Some(
            "http://www.bing.com/news/apiclick.aspx?ref=FexRss&aid=&tid=12BE3CE268B0484F92DD9828C685E325&url=https%3a%2f%2fwww.dailymail.co.uk%2fsport%2ffootball%2farticle-9554407%2fReal-Madrid-Barcelona-Juventus-threaten-clubs-withdrawing-Super-League-legal-action.html&c=4438839993362862681&mkt=it-it",
        ),
        summary_prefix: Some(
            "The three European giants are refusing to let the plans die and have",
        ),
        ..Case::DEFAULT
    }
    .check();
}
