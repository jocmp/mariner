//! `feed-test-source.xml`.

use crate::Case;

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-source.xml",
        title: Some("À la une - Google Actualités"),
        site_url: Some("https://news.google.com/?hl=fr&gl=BE&ceid=BE:fr"),
        description: Some("Google Actualités"),
        entry_id: Some("52782335008021"),
        entry_title: Some(
            "Coronavirus en Belgique: 200 cas détectés au total, un nombre limité de tests effectués par manque de réactifs - RTBF",
        ),
        entry_url: Some(
            "https://news.google.com/__i/rss/rd/articles/CBMiiwFodHRwczovL3d3dy5ydGJmLmJlL2luZm8vc29jaWV0ZS9kZXRhaWxfY29yb25hdmlydXMtZW4tYmVsZ2lxdWUtMjAwLWNhcy1kZXRlY3Rlcy1hdS10b3RhbC1sYS1jaXJjdWxhdGlvbi1kdS12aXJ1cy1yZXN0ZS1saW1pdGVlP2lkPTEwNDUwNzkz0gEA?oc=5",
        ),
        summary_prefix: Some(
            "<ol><li><a href=\"https://news.google.com/__i/rss/rd/articles/CBMiiwFodHRwczovL3d",
        ),
        ..Case::DEFAULT
    }
    .check();
}
