//! Media RSS and enclosures, which the reference output doesn't cover
//! beyond YouTube.

use mariner_feeds::{Enclosure, Entry, MediaContent, MediaThumbnail, parse};

#[test]
fn reads_grouped_media_rss() {
    let entries = entries("media-rss.xml");
    let media = entries[0].media.clone().unwrap();
    assert_eq!(media.title.as_deref(), Some("Group title"));
    assert_eq!(media.description.as_deref(), Some("Group & description"));
    assert_eq!(
        media.contents,
        [
            MediaContent {
                url: "https://example.org/clip.mp4".into(),
                mime_type: Some("video/mp4".into()),
                medium: Some("video".into()),
                width: Some("1280".into()),
                height: Some("720".into()),
                file_size: Some("1000".into()),
                duration: Some("61".into()),
                title: Some("Clip title".into()),
                description: Some("Clip description".into()),
                group: Some(0),
                is_default: false,
            },
            MediaContent {
                url: "https://example.org/clip.mp3".into(),
                mime_type: Some("audio/mpeg".into()),
                medium: Some("audio".into()),
                width: None,
                height: None,
                file_size: None,
                duration: None,
                title: None,
                description: None,
                group: Some(0),
                is_default: false,
            },
        ],
        "a media:content without a url is skipped"
    );
    assert_eq!(
        media.thumbnails,
        [
            thumbnail("https://example.org/clip.jpg", Some("320"), Some("180")),
            thumbnail("https://example.org/second.jpg", None, None),
        ]
    );
    assert_eq!(media.star_average.as_deref(), Some("4.5"));
    assert_eq!(media.star_count.as_deref(), Some("12"));
    assert_eq!(media.views.as_deref(), Some("345"));

    assert_eq!(entries[1].media, None);
}

#[test]
fn numbers_groups_and_marks_defaults() {
    let media = entries("media-groups.xml")[0].media.clone().unwrap();
    let contents: Vec<_> = media
        .contents
        .iter()
        .map(|content| (content.url.as_str(), content.group, content.is_default))
        .collect();
    assert_eq!(
        contents,
        [
            ("https://example.org/a-1080.mp4", Some(0), false),
            ("https://example.org/a-480.mp4", Some(0), true),
            ("https://example.org/b.webm", Some(1), true),
            ("https://example.org/b.mp4", Some(1), false),
            ("https://example.org/poster.jpg", None, false),
        ]
    );
    assert_eq!(media.title.as_deref(), Some("Second video"));
}

#[test]
fn lists_every_enclosure_without_changing_the_reference_fields() {
    let entry = &entries("media-rss.xml")[0];
    assert_eq!(
        entry.enclosures,
        [
            enclosure(
                "https://example.org/episode.mp3",
                Some("audio/mpeg"),
                Some("2000")
            ),
            enclosure("https://example.org/cover.jpg", Some("image/jpeg"), None),
        ]
    );
    // Outside podcast feeds, the reference reads the first enclosure as the
    // image (it comes after the first media:content and media:thumbnail,
    // so it wins) and sets no enclosure.
    assert_eq!(entry.enclosure, None);
    assert_eq!(
        entry.image.as_deref(),
        Some("https://example.org/episode.mp3")
    );
}

#[test]
fn reads_media_in_atom() {
    let entry = &entries("media-atom.xml")[0];
    let media = entry.media.clone().unwrap();
    assert_eq!(
        media.thumbnails,
        [thumbnail(
            "https://example.org/1.jpg",
            Some("640"),
            Some("360")
        )]
    );
    assert_eq!(media.contents[0].url, "https://example.org/1.png");
    assert_eq!(media.contents[0].description.as_deref(), Some("Alt text"));
    assert_eq!(
        entry.enclosures,
        [enclosure(
            "https://example.org/1.mp3",
            Some("audio/mpeg"),
            Some("3000")
        )]
    );
}

fn entries(fixture: &str) -> Vec<Entry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/features");
    parse(&std::fs::read(format!("{path}/{fixture}")).unwrap())
        .unwrap()
        .entries
}

fn thumbnail(url: &str, width: Option<&str>, height: Option<&str>) -> MediaThumbnail {
    MediaThumbnail {
        url: url.into(),
        width: width.map(String::from),
        height: height.map(String::from),
    }
}

fn enclosure(url: &str, mime_type: Option<&str>, length: Option<&str>) -> Enclosure {
    Enclosure {
        url: url.into(),
        mime_type: mime_type.map(String::from),
        length: length.map(String::from),
    }
}
