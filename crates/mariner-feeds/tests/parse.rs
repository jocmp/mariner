//! Behavior checks that don't need reference output.

use mariner_feeds::{Entry, Error, Format, parse};

const FEED_URL: &str = "https://example.org/feed.xml";

#[test]
fn public_id_hashes_feed_url_and_entry_id() {
    let entry = Entry {
        id: Some("item-1".into()),
        title: Some("Ignored when there is an id".into()),
        ..Entry::default()
    };
    assert_eq!(
        entry.public_id(FEED_URL),
        "1e6f649ea7f8aedfbc39ddf457d8b3f84bcfe00f"
    );
}

#[test]
fn public_id_without_entry_id_uses_url_date_and_title() {
    let entry = Entry {
        url: Some("https://example.org/p".into()),
        published: parse_time("2024-10-05T10:11:12Z"),
        title: Some("Title".into()),
        ..Entry::default()
    };
    assert_eq!(
        entry.public_id(FEED_URL),
        "e1075b15f753529801a1f9be44afcb448fa3ec78"
    );

    let title_only = Entry {
        title: Some("Title".into()),
        ..Entry::default()
    };
    assert_eq!(
        title_only.public_id(FEED_URL),
        "d23e819d2b998d66db99680970c6a56b0d62d7e9"
    );
}

#[test]
fn detects_each_format() {
    let rss = br#"<rss version="2.0"><channel><title>R</title></channel></rss>"#;
    let atom = br#"<feed xmlns="http://www.w3.org/2005/Atom"><title>A</title></feed>"#;
    let json = br#"{"version": "https://jsonfeed.org/version/1.1", "title": "J", "items": []}"#;
    assert_eq!(parse(rss).unwrap().format, Format::Rss);
    assert_eq!(parse(atom).unwrap().format, Format::Atom);
    assert_eq!(parse(json).unwrap().format, Format::Json);
}

#[test]
fn rejects_documents_that_are_not_feeds() {
    let html = b"<!doctype html><html><head><title>Page</title></head></html>";
    assert!(matches!(parse(html), Err(Error::NotAFeed)));
    assert!(matches!(parse(b"{}"), Err(Error::NotAFeed)));
}

#[test]
fn reads_rss_items() {
    let feed = parse(
        br#"<?xml version="1.0"?>
        <rss version="2.0"><channel>
          <title>Feed</title>
          <link>https://example.org/</link>
          <item>
            <title> Hello </title>
            <guid isPermaLink="false">g1</guid>
            <pubDate>Sat, 05 Oct 2024 10:11:12 +0200</pubDate>
            <dc:date>2024-10-06T00:00:00Z</dc:date>
            <category> news </category>
          </item>
          <item><guid>https://example.org/2</guid></item>
        </channel></rss>"#,
    )
    .unwrap();
    assert_eq!(feed.title.as_deref(), Some("Feed"));
    assert_eq!(feed.site_url.as_deref(), Some("https://example.org/"));

    let first = &feed.entries[0];
    assert_eq!(first.title.as_deref(), Some("Hello"));
    assert_eq!(first.id.as_deref(), Some("g1"));
    assert_eq!(first.url, None, "a guid that isn't a permalink isn't a URL");
    assert_eq!(first.published, parse_time("2024-10-05T08:11:12Z"));
    assert_eq!(first.categories, ["news"]);

    let second = &feed.entries[1];
    assert_eq!(second.url.as_deref(), Some("https://example.org/2"));
}

#[test]
fn keeps_what_precedes_a_well_formedness_error() {
    let feed = parse(
        b"<rss><channel><title>T</title>\
          <item><title>Kept</title></item>\
          <item><title>Lost&nbsp;</title></item>\
          </channel></rss>",
    )
    .unwrap();
    let titles: Vec<_> = feed.entries.iter().map(|e| e.title.as_deref()).collect();
    assert_eq!(titles, [Some("Kept")]);
}

#[test]
fn decodes_the_declared_encoding() {
    let mut bytes =
        b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><rss><channel><title>Caf".to_vec();
    bytes.push(0xE9);
    bytes.extend_from_slice(b"</title></channel></rss>");
    assert_eq!(parse(&bytes).unwrap().title.as_deref(), Some("Café"));
}

fn parse_time(text: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    Some(text.parse().unwrap())
}
