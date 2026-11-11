//! `feed-rdf-distrowatch.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-rdf-distrowatch.xml",
        title: Some("DistroWatch.com: DistroWatch Weekly"),
        site_url: Some("https://distrowatch.com/"),
        description: Some("Latest news on Linux distributions and BSD projects"),
        entry_title: Some("DistroWatch Weekly, Issue 1084"),
        entry_url: Some("https://distrowatch.com/weekly.php?issue=20240819"),
        summary_prefix: Some(
            "The DistroWatch Weekly news feed is brought to you by <a href=\"https://www.tuxed",
        ),
        ..Case::DEFAULT
    }
    .check();
}
