//! `atom-relative-image.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-relative-image.xml",
        title: Some("Crystal's Wobsite: Articles"),
        site_url: Some("https://crystalwobsite.gay"),
        entry_id: Some("https://crystalwobsite.gay/posts/2025-10-03-fluentflame.html"),
        entry_title: Some("Working on Fluentflame Reader"),
        entry_url: Some("https://crystalwobsite.gay/posts/2025-10-03-fluentflame"),
        summary_prefix: Some(
            "Small blogpost because I don’t feel like writing a long one, but I felt I",
        ),
        content_prefix: Some(
            "<p>Small blogpost because I don’t feel like writing a long one, but I felt I",
        ),
        ..Case::DEFAULT
    }
    .check();
}
