//! Atom 1.0 and 0.3.

use std::sync::LazyLock;

use regex::Regex;

use super::Parser;
use crate::model::Format;
use crate::rules::{Class, Field, HUB, SELF, TEXT_HTML, attr, attr_with, attrs, collection, text};
use crate::sax::Parsed;
use crate::xhtml;

/// An Atom entry's rules: the ones every Atom entry shares (with the given
/// link rules), then the parser's own, then a plain `content` element.
macro_rules! atom_entry {
    ([$($links:expr),*]; $($extra:expr),* $(,)?) => {
        &[
            $crate::rules::text_with("title", $crate::rules::Field::RawTitle, &[("type", Some("html"))]),
            $crate::rules::text_with("title", $crate::rules::Field::RawTitle, &[("type", Some("xhtml"))]),
            $crate::rules::text_with("title", $crate::rules::Field::RawTitle, &[("type", Some("xml"))]),
            $crate::rules::text_with("title", $crate::rules::Field::Title, &[("type", Some("text"))]),
            $crate::rules::text_with("title", $crate::rules::Field::Title, &[("type", None)]),
            $crate::rules::attr("title", $crate::rules::Field::TitleType, "type"),
            $crate::rules::text("name", $crate::rules::Field::Author),
            $crate::rules::text("content", $crate::rules::Field::Content),
            $crate::rules::text("summary", $crate::rules::Field::Summary),
            $crate::rules::attr("enclosure", $crate::rules::Field::Image, "href"),
            $crate::rules::text("published", $crate::rules::Field::Published),
            $crate::rules::text("id", $crate::rules::Field::EntryId),
            $crate::rules::text("created", $crate::rules::Field::Published),
            $crate::rules::text("issued", $crate::rules::Field::Published),
            $crate::rules::text("updated", $crate::rules::Field::Updated),
            $crate::rules::text("modified", $crate::rules::Field::Updated),
            $crate::rules::attrs("category", $crate::rules::Field::Categories, "term", &[]),
            $($links,)*
            $($extra,)*
            $crate::rules::text("content", $crate::rules::Field::Content),
        ]
    };
}
pub(super) use atom_entry;

pub(super) struct Atom;

static ATOM: Class = Class::new(&[
    text("title", Field::Title),
    text("subtitle", Field::Description),
    attr_with("link", Field::Url, "href", TEXT_HTML),
    attr_with("link", Field::FeedUrl, "href", SELF),
    attrs("link", Field::Links, "href", &[]),
    attrs("link", Field::Hubs, "href", HUB),
    collection("entry", Field::Entries, &ATOM_ENTRY),
    text("icon", Field::Icon),
]);

static ATOM_ENTRY: Class = Class::new(atom_entry!(
    [
        attr_with("link", Field::Url, "href", &[("type", Some("text/html")), ("rel", Some("alternate"))]),
        attrs("link", Field::Links, "href", &[])
    ];
    attr("media:thumbnail", Field::Image, "url"),
    attr("media:content", Field::Image, "url"),
));

impl Parser for Atom {
    /// A `<feed>` declaring the Atom namespace as its default.
    fn able_to_parse(&self, text: &str) -> bool {
        static FEED: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"<feed[^>]+xmlns\s?=\s?["'](https?://www\.w3\.org/2005/Atom|http://purl\.org/atom/ns\#)["'][^>]*>"#,
            )
            .unwrap()
        });
        FEED.is_match(text)
    }

    fn format(&self) -> Format {
        Format::Atom
    }

    fn feed(&self) -> &'static Class {
        &ATOM
    }

    fn preprocesses(&self) -> bool {
        true
    }

    /// The HTML link, or else the last link that isn't the feed's own.
    fn site_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        let self_url = self.self_url(feed);
        feed.single(Field::Url).or_else(|| {
            feed.multi(Field::Links)
                .filter(|link| *link != self_url)
                .last()
                .flatten()
        })
    }

    fn self_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        feed.single(Field::FeedUrl)
    }

    fn entry_title(&self, entry: &Parsed) -> Option<String> {
        entry_title(entry)
    }

    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry_url(entry)
    }
}

/// An HTML-typed title as plain text, else the title as written.
pub(super) fn entry_title(entry: &Parsed) -> Option<String> {
    match entry.single(Field::RawTitle) {
        Some(raw) => Some(xhtml::text_content(raw)),
        None => entry.single(Field::Title).map(String::from),
    }
}

/// The alternate HTML link, or else the first link.
pub(super) fn entry_url<'a>(entry: &Parsed<'a>) -> Option<&'a str> {
    entry
        .single(Field::Url)
        .or_else(|| entry.multi(Field::Links).next().flatten())
}
