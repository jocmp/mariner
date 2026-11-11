//! `feed-multiple-media-content.xml`.

use crate::{Case, first_entry};

#[test]
fn first_entry_matches() {
    Case {
        fixture: "feed-multiple-media-content.xml",
        title: Some("Nuclearpasta"),
        site_url: Some("https://mastodon.art/@nuclearpasta"),
        description: Some("Public posts from @nuclearpasta@mastodon.art"),
        entry_id: Some("https://mastodon.art/@nuclearpasta/114495761108890085"),
        entry_url: Some("https://mastodon.art/@nuclearpasta/114495761108890085"),
        categories: Some(&["webcomicday", "webcomic", "fantasy", "comics"]),
        summary_prefix: Some(
            "<p>Oh it&#39;s <a href=\"https://mastodon.art/tags/WebcomicDay\" class=\"mention ha",
        ),
        ..Case::DEFAULT
    }
    .check();
}

#[test]
fn media_contents() {
    let media = first_entry("feed-multiple-media-content.xml")
        .media
        .unwrap();
    let contents: Vec<_> = media
        .contents
        .iter()
        .map(|content| (content.url.as_str(), content.mime_type.as_deref()))
        .collect();
    let files = "https://cdn.masto.host/mastodonart/media_attachments/files/114/495";
    assert_eq!(
        contents,
        [
            (
                &*format!("{files}/754/804/385/848/original/dee105af83a73921.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/755/611/292/322/original/c332f57157abc203.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/756/453/333/085/original/f0baf8d5c4be0461.jpg"),
                Some("image/jpeg")
            ),
            (
                &*format!("{files}/757/644/563/056/original/884111c48e491831.jpg"),
                Some("image/jpeg")
            ),
        ]
    );
    assert_eq!(
        media.contents[0].description.as_deref(),
        Some("Raiders ride giant lizards across the desert.")
    );
}
