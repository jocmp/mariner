//! `feed-test-image-link.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-image-link.xml",
        title: Some("Bleacher Report - Front Page"),
        site_url: Some("https://bleacherreport.com"),
        description: Some(
            "Bleacher Report - The latest articles about sports news, rumors, teams, events, and more.",
        ),
        entry_id: Some(
            "https://bleacherreport.com/articles/10077424-orange-cassidy-retains-aew-international-title-at-double-or-nothing-2023",
        ),
        entry_title: Some(
            "Orange Cassidy Retains AEW International Title at Double or Nothing 2023",
        ),
        entry_url: Some(
            "https://bleacherreport.com/articles/10077424-orange-cassidy-retains-aew-international-title-at-double-or-nothing-2023",
        ),
        categories: Some(&[
            "Breaking News",
            "All Elite Wrestling",
            "BNT MISC",
            "Orange Cassidy",
        ]),
        summary_prefix: Some(
            "Orange Cassidy won a 21-man Blackjack Battle Royal at Double or Nothing on Sunda",
        ),
        ..Case::DEFAULT
    }
    .check();
}
