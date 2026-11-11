//! `feed-test-image-channel-reverse.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-test-image-channel-reverse.xml",
        title: Some("The Joe Rogan Experience"),
        site_url: Some("https://www.joerogan.com"),
        description: Some("The podcast of Comedian Joe Rogan.."),
        entry_id: Some("0d7147a3-f1c1-4ae6-bbf8-2e0a493639ca"),
        entry_title: Some("#1405 - Sober October 3 Recap"),
        entry_url: Some("http://traffic.libsyn.com/joeroganexp/p1405.mp3"),
        summary_prefix: Some(
            "Joe is joined by Ari Shaffir, Bert Kreischer & Tom Segura to recap their 3rd ann",
        ),
        content_prefix: Some(
            "Joe is joined by Ari Shaffir, Bert Kreischer & Tom Segura to recap their 3rd ann",
        ),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn audio() {
    let entry = first_entry("feed-test-image-channel-reverse.xml");
    let url = "http://traffic.libsyn.com/joeroganexp/p1405.mp3?dest-id=19997";
    assert_eq!(
        entry.enclosure.map(|enclosure| enclosure.url).as_deref(),
        Some(url)
    );
    assert_eq!(entry.enclosures[0].url, url);
}
