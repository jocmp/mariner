//! RSS served through FeedBurner, whose items keep the original link in
//! `feedburner:origLink`.

use super::Parser;
use super::rss::{self, rss_entry};
use crate::model::Format;
use crate::rules::{Class, Field, HUB, attr, attrs, collection, text};
use crate::sax::Parsed;

pub(super) struct RssFeedBurner;

static RSS_FEED_BURNER: Class = Class::new(&[
    text("title", Field::Title),
    text("description", Field::Description),
    text("link", Field::Url),
    attrs("atom10:link", Field::Hubs, "href", HUB),
    collection("item", Field::Entries, &RSS_FEED_BURNER_ENTRY),
]);

static RSS_FEED_BURNER_ENTRY: Class = Class::new(rss_entry!(
    [attr("enclosure", Field::Image, "url")];
    text("feedburner:origLink", Field::OrigLink),
));

impl Parser for RssFeedBurner {
    fn able_to_parse(&self, text: &str) -> bool {
        rss::is_rss(text) && text.contains("feedburner")
    }

    fn format(&self) -> Format {
        Format::Rss
    }

    fn feed(&self) -> &'static Class {
        &RSS_FEED_BURNER
    }

    fn entry_id<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        rss::entry_id(entry)
    }

    /// The original link, or else the RSS item's URL.
    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        entry
            .single(Field::OrigLink)
            .or_else(|| rss::entry_url(entry))
    }
}
