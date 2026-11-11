//! The XML feed parsers, one module per kind of feed, tried in order.
//!
//! Each parser says whether it can read a document, declares the classes
//! of objects it builds (with the vocabulary in `rules`), and reads the
//! fields that need more than one rule (a feed's site URL, an entry's URL).
//! `sax` runs them.

use crate::model::{Format, Itunes, Youtube};
use crate::rules::{Class, Field};
use crate::sax::Parsed;

mod atom;
mod atom_feed_burner;
mod atom_youtube;
mod globally_unique_identifier;
mod itunes_rss;
mod itunes_rss_category;
mod itunes_rss_owner;
mod podlove_chapter;
mod rss;
mod rss_feed_burner;
mod rss_image;

/// Tried in this order; the first that can parse a document reads it.
static PARSERS: [&(dyn Parser + Sync); 6] = [
    &itunes_rss::ItunesRss,
    &rss_feed_burner::RssFeedBurner,
    &atom_youtube::AtomYoutube,
    &atom_feed_burner::AtomFeedBurner,
    &atom::Atom,
    &rss::Rss,
];

/// The parser for `text`, or `None` when it isn't an XML feed.
pub(crate) fn detect(text: &str) -> Option<&'static dyn Parser> {
    PARSERS
        .iter()
        .find(|parser| parser.able_to_parse(text))
        .map(|parser| *parser as &dyn Parser)
}

pub(crate) trait Parser {
    /// Whether this parser reads `text`, judged from the raw text.
    fn able_to_parse(&self, text: &str) -> bool;

    fn format(&self) -> Format;

    /// The class of the document's root object.
    fn feed(&self) -> &'static Class;

    /// Whether to drop namespace prefixes and turn XHTML content into HTML
    /// text before parsing.
    fn preprocesses(&self) -> bool {
        false
    }

    fn site_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        feed.single(Field::Url)
    }

    fn self_url<'a>(&self, _feed: &Parsed<'a>) -> Option<&'a str> {
        None
    }

    fn entry_id<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry.single(Field::EntryId)
    }

    fn entry_title(&self, entry: &Parsed) -> Option<String> {
        entry.single(Field::Title).map(String::from)
    }

    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry.single(Field::Url)
    }

    fn itunes(&self, _entry: &Parsed) -> Option<Itunes> {
        None
    }

    fn youtube(&self, _entry: &Parsed) -> Option<Youtube> {
        None
    }
}
