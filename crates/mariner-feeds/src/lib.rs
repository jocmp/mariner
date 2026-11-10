//! Parses RSS, Atom, RDF and JSON Feed documents into a common model, and
//! finds the feeds a web page links to (FeedFinder).

mod date;
mod decode;
mod json_feed;
mod model;
mod rules;
mod sax;
mod tree;
mod xhtml;

pub use model::{Enclosure, Entry, Feed, Format, Itunes};

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
    if let Some(flavor) = rules::detect(text) {
        return Ok(sax::parse(text, flavor, decoded.declared_encoding));
    }
    if json_feed::is_json_feed(text) {
        return json_feed::parse(text);
    }
    Err(Error::NotAFeed)
}
