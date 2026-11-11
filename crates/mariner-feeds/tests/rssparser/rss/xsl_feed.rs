//! `feed-test-xsl.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-xsl.xml",
        title: Some("SkySports | Liverpool"),
        site_url: Some("http://www.skysports.com"),
        description: Some("Liverpool News"),
        entry_title: Some("Insight: Who should Liverpool sign?"),
        entry_url: Some(
            "https://www.skysports.com/football/news/11669/11719097/premier-league-transfer-window-who-should-liverpool-sign",
        ),
        categories: Some(&["News Story"]),
        summary_prefix: Some(
            "Liverpool just missed out on clinching the Premier League title and have a Champ",
        ),
        ..Case::DEFAULT
    }
    .check();
}
