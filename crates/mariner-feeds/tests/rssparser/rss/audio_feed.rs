//! `feed-test-audio.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-audio.xml",
        title: Some("Stuff You Should Know"),
        site_url: Some("https://www.howstuffworks.com"),
        description: Some(
            "If you've ever wanted to know about champagne, satanism, the Stonewall Uprising, chaos theory, LSD, El Nino, true crime and Rosa Parks, then look no further. Josh and Chuck have you covered.",
        ),
        entry_id: Some("c41042ae-5460-11e8-b38c-5f5faf9dd0c5"),
        entry_title: Some("SYSK Selects: How Gold Works"),
        summary_prefix: Some(
            "As of early 2013, only 161,00 metric tons of gold had been mined in the entire h",
        ),
        content_prefix: Some(
            "<p>As of early 2013, only 161,00 metric tons of gold had been mined in the entir",
        ),
        ..Case::DEFAULT
    }
    .check();
}
