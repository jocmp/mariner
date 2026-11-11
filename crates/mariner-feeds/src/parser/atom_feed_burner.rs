//! Atom served through FeedBurner.

use super::Parser;
use super::atom::{self, atom_entry};
use crate::model::Format;
use crate::rules::{Class, Field, HUB, attr, attr_with, attrs, collection, text};
use crate::sax::Parsed;

pub(super) struct AtomFeedBurner;

const ATOM_XML: crate::rules::With = &[("type", Some("application/atom+xml"))];

static ATOM_FEED_BURNER: Class = Class::new(&[
    text("title", Field::Title),
    text("subtitle", Field::Description),
    attr_with(
        "link",
        Field::UrlTextHtml,
        "href",
        &[("type", Some("text/html"))],
    ),
    attr_with("link", Field::UrlNotype, "href", &[("type", None)]),
    attr_with("link", Field::FeedUrlLink, "href", ATOM_XML),
    attr_with("atom10:link", Field::FeedUrlAtom10Link, "href", ATOM_XML),
    attrs("atom10:link", Field::Hubs, "href", HUB),
    collection("entry", Field::Entries, &ATOM_FEED_BURNER_ENTRY),
]);

static ATOM_FEED_BURNER_ENTRY: Class = Class::new(atom_entry!(
    [
        attr_with("link", Field::Url, "href", &[("type", Some("text/html")), ("rel", Some("alternate"))]),
        attrs("link", Field::Links, "href", &[])
    ];
    text("feedburner:origLink", Field::OrigLink),
    attr("media:thumbnail", Field::Image, "url"),
    attr("media:content", Field::Image, "url"),
));

impl Parser for AtomFeedBurner {
    fn able_to_parse(&self, text: &str) -> bool {
        text.contains("<feed")
            && text.contains("Atom")
            && text.contains("feedburner")
            && !super::rss::is_rss(text)
    }

    fn format(&self) -> Format {
        Format::Atom
    }

    fn feed(&self) -> &'static Class {
        &ATOM_FEED_BURNER
    }

    fn preprocesses(&self) -> bool {
        true
    }

    /// The `text/html` link, or else a link with no type.
    fn site_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        feed.single(Field::UrlTextHtml)
            .or_else(|| feed.single(Field::UrlNotype))
    }

    /// The Atom-typed `link`, or else the Atom-typed `atom10:link`.
    fn self_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        feed.single(Field::FeedUrlLink)
            .or_else(|| feed.single(Field::FeedUrlAtom10Link))
    }

    fn entry_title(&self, entry: &Parsed) -> Option<String> {
        atom::entry_title(entry)
    }

    /// The original link, or else the Atom entry's URL. (Preprocessing
    /// strips the `feedburner:` prefix, so the original link never matches.)
    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry
            .single(Field::OrigLink)
            .or_else(|| atom::entry_url(entry))
    }
}
