//! `feed-test-unexpected-token.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-unexpected-token.xml",
        title: Some("Wheels Off-Road & 4x4"),
        site_url: Some("https://www.wheels24.co.za/"),
        entry_title: Some(
            "Wheels24.co.za | WATCH | Range Rover spices up its Velar line-up, adds new hybrid model to the range",
        ),
        entry_url: Some(
            "https://www.wheels24.co.za/OffRoad_and_4x4/Bakkie_and_SUV/watch-range-rover-spices-up-its-velar-line-up-adds-new-hybrid-model-to-the-range-20200923-3",
        ),
        summary_prefix: Some(
            "Range Rover updated its desirable Velar range by adding new engine choices and a",
        ),
        ..Case::DEFAULT
    }
    .check();
}
