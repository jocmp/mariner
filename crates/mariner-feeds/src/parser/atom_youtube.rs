//! YouTube's Atom feeds.

use super::Parser;
use super::atom::{self, atom_entry};
use crate::model::{Format, Youtube};
use crate::rules::{ALTERNATE, Class, Field, SELF, attr, attr_with, collection, text};
use crate::sax::Parsed;

pub(super) struct AtomYoutube;

static ATOM_YOUTUBE: Class = Class::new(&[
    text("title", Field::Title),
    attr_with("link", Field::Url, "href", ALTERNATE),
    attr_with("link", Field::FeedUrl, "href", SELF),
    text("name", Field::Other),
    text("yt:channelId", Field::YoutubeChannelId),
    collection("entry", Field::Entries, &ATOM_YOUTUBE_ENTRY),
]);

/// An Atom entry whose only link rule is the alternate link.
static ATOM_YOUTUBE_ENTRY: Class = Class::new(atom_entry!(
    [];
    attr_with("link", Field::Url, "href", ALTERNATE),
    text("media:description", Field::Content),
    text("yt:videoId", Field::YoutubeVideoId),
    text("yt:channelId", Field::YoutubeChannelId),
    text("media:title", Field::Other),
    attr("media:content", Field::Other, "url"),
    attr("media:thumbnail", Field::Other, "url"),
));

impl Parser for AtomYoutube {
    fn able_to_parse(&self, text: &str) -> bool {
        text.contains(r#"xmlns:yt="http://www.youtube.com/xml/schemas/2015""#)
    }

    fn format(&self) -> Format {
        Format::Atom
    }

    fn feed(&self) -> &'static Class {
        &ATOM_YOUTUBE
    }

    fn self_url<'a>(&self, feed: &Parsed<'a>) -> Option<&'a str> {
        feed.single(Field::FeedUrl)
    }

    fn entry_title(&self, entry: &Parsed) -> Option<String> {
        atom::entry_title(entry)
    }

    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        atom::entry_url(entry)
    }

    fn youtube(&self, entry: &Parsed) -> Option<Youtube> {
        Some(Youtube {
            video_id: entry.single(Field::YoutubeVideoId).map(String::from),
            channel_id: entry.single(Field::YoutubeChannelId).map(String::from),
        })
    }
}
