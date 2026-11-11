//! `atom-image-query-params.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-image-query-params.xml",
        title: Some("9to5Google"),
        site_url: Some("https://9to5google.com/"),
        description: Some("Google news, Pixel, Android, Home, Chrome OS, more"),
        entry_id: Some("https://9to5google.com/2024/09/02/labor-day-android-game-app-deals/"),
        entry_title: Some(
            "Labor Day Android game and app price drops: SOULVARS, Bloons TD 6, Space Grunts 2, more",
        ),
        entry_author: Some("9to5Toys"),
        entry_url: Some("https://9to5toys.com/2024/09/02/labor-day-android-game-app-deals/"),
        categories: Some(&["Deals"]),
        summary_prefix: Some(
            "<div class=\"feat-image\"><img src=\"https://9to5google.com/wp-content/uploads/site",
        ),
        ..Case::DEFAULT
    }
    .check();
}
