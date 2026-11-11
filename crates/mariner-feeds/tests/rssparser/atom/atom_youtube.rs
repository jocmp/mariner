//! `atom-feed-youtube.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "atom-feed-youtube.xml",
        title: Some("Digital Foundry"),
        site_url: Some("https://www.youtube.com/channel/UC9PBzalIcEQCsiIkq36PyUA"),
        entry_id: Some("yt:video:W_S7iKZZdHw"),
        entry_title: Some("Digital Foundry Games of 2024: The Oliver Mackenzie Collection!"),
        entry_author: Some("Digital Foundry"),
        entry_url: Some("https://www.youtube.com/watch?v=W_S7iKZZdHw"),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn media() {
    let entry = first_entry("atom-feed-youtube.xml");
    let media = entry.media.unwrap();
    assert_eq!(
        entry.youtube.unwrap().video_id.as_deref(),
        Some("W_S7iKZZdHw")
    );
    assert_eq!(
        media.title.as_deref(),
        Some("Digital Foundry Games of 2024: The Oliver Mackenzie Collection!")
    );
    assert_eq!(
        media.contents[0].url,
        "https://www.youtube.com/v/W_S7iKZZdHw?version=3"
    );
    assert_eq!(
        media.thumbnails[0].url,
        "https://i4.ytimg.com/vi/W_S7iKZZdHw/hqdefault.jpg"
    );
    assert!(media.description.unwrap().starts_with(
        "As the holiday period begins to unwind, we continue to look back on an excellent 2024"
    ));
}
