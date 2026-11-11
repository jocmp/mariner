//! Parses RSS, Atom, RDF and JSON Feed documents into a common model, and
//! finds the feeds a web page links to (FeedFinder).

mod date;
mod decode;
pub mod finder;
mod json_feed;
mod media;
mod model;
mod parser;
mod rules;
mod sax;
mod tree;
mod xhtml;
mod xml_feed;

pub use model::{
    Enclosure, Entry, Feed, Format, Itunes, Media, MediaContent, MediaThumbnail, Youtube,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a feed")]
    NotAFeed,
}

/// Parses a feed document from its raw bytes. XML documents are decoded by
/// their byte order mark or declared encoding, falling back to UTF-8.
pub fn parse(bytes: &[u8]) -> Result<Feed, Error> {
    parse_with_charset(bytes, None)
}

/// Like [`parse`], with a charset from outside the document (an HTTP
/// `Content-Type` header, say). The hint applies only when the document
/// has no byte order mark and declares no encoding of its own.
pub fn parse_with_charset(bytes: &[u8], charset: Option<&str>) -> Result<Feed, Error> {
    let decoded = decode::decode(bytes, charset);
    let text = decoded.text.trim_start();
    let feed = if let Some(parser) = parser::detect(text) {
        xml_feed::parse(text, parser, decoded.declared_encoding)
    } else if json_feed::is_json_feed(text) {
        json_feed::parse(text)?
    } else {
        return Err(Error::NotAFeed);
    };
    // A page that only mentions `<rss` isn't a feed.
    let has_content = feed.title.is_some()
        || feed.site_url.is_some()
        || feed.self_url.is_some()
        || !feed.entries.is_empty();
    if has_content {
        Ok(feed)
    } else {
        Err(Error::NotAFeed)
    }
}
