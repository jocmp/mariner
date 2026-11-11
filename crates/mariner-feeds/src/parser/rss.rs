//! RSS 0.9x, 1.0 (RDF) and 2.0.

use super::Parser;
use super::globally_unique_identifier;
use super::rss_image::RSS_IMAGE;
use crate::model::Format;
use crate::rules::{Class, Field, HUB, attr, attrs, collection, object, text};
use crate::sax::Parsed;

/// An RSS item's rules: the ones every RSS item shares (with or without
/// reading `enclosure` as the image), then the parser's own, then a plain
/// `content` element.
macro_rules! rss_entry {
    ([$($enclosure:expr)?]; $($extra:expr),* $(,)?) => {
        &[
            $crate::rules::text("title", $crate::rules::Field::Title),
            $crate::rules::text("content:encoded", $crate::rules::Field::Content),
            $crate::rules::text("a10:content", $crate::rules::Field::Content),
            $crate::rules::text("description", $crate::rules::Field::Summary),
            $crate::rules::text("link", $crate::rules::Field::Url),
            $crate::rules::attr("a10:link", $crate::rules::Field::Url, "href"),
            $crate::rules::text("author", $crate::rules::Field::Author),
            $crate::rules::text("dc:creator", $crate::rules::Field::Author),
            $crate::rules::text("a10:name", $crate::rules::Field::Author),
            $crate::rules::text("pubDate", $crate::rules::Field::Published),
            $crate::rules::text("pubdate", $crate::rules::Field::Published),
            $crate::rules::text("issued", $crate::rules::Field::Published),
            $crate::rules::text("dc:date", $crate::rules::Field::Published),
            $crate::rules::text("dc:Date", $crate::rules::Field::Published),
            $crate::rules::text("dcterms:created", $crate::rules::Field::Published),
            $crate::rules::text("dcterms:modified", $crate::rules::Field::Updated),
            $crate::rules::text("a10:updated", $crate::rules::Field::Updated),
            $crate::rules::object(
                "guid",
                $crate::rules::Field::EntryId,
                &$crate::parser::globally_unique_identifier::GLOBALLY_UNIQUE_IDENTIFIER,
            ),
            $crate::rules::text("dc:identifier", $crate::rules::Field::Other),
            $crate::rules::attr("media:thumbnail", $crate::rules::Field::Image, "url"),
            $crate::rules::attr("media:content", $crate::rules::Field::Image, "url"),
            $($enclosure,)?
            $crate::rules::text("comments", $crate::rules::Field::Other),
            $crate::rules::texts("category", $crate::rules::Field::Categories),
            $($extra,)*
            $crate::rules::text("content", $crate::rules::Field::Content),
        ]
    };
}
pub(super) use rss_entry;

pub(super) struct Rss;

static RSS: Class = Class::new(&[
    text("description", Field::Description),
    object("image", Field::Image, &RSS_IMAGE),
    text("language", Field::Language),
    text("link", Field::Url),
    attr("a10:link", Field::Url, "href"),
    text("title", Field::Title),
    attrs("atom:link", Field::Hubs, "href", HUB),
    collection("item", Field::Entries, &RSS_ENTRY),
]);

static RSS_ENTRY: Class = Class::new(rss_entry!([attr("enclosure", Field::Image, "url")];));

impl Parser for Rss {
    fn able_to_parse(&self, text: &str) -> bool {
        is_rss(text) && !text.contains("feedburner")
    }

    fn format(&self) -> Format {
        Format::Rss
    }

    fn feed(&self) -> &'static Class {
        &RSS
    }

    fn entry_id<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry_id(entry)
    }

    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry_url(entry)
    }
}

pub(super) fn is_rss(text: &str) -> bool {
    text.contains("<rss") || text.contains("<rdf")
}

/// The GUID's text.
pub(super) fn entry_id<'a>(entry: &Parsed<'a>) -> Option<&'a str> {
    entry
        .object(Field::EntryId)
        .and_then(|guid| globally_unique_identifier::guid(&guid))
}

/// The link, or else a GUID that's a permalink.
pub(super) fn entry_url<'a>(entry: &Parsed<'a>) -> Option<&'a str> {
    entry.single(Field::Url).or_else(|| {
        entry
            .object(Field::EntryId)
            .and_then(|guid| globally_unique_identifier::url(&guid))
    })
}
