//! RSS 2.0 with Apple's podcast (iTunes) extensions.

use std::sync::LazyLock;

use regex::Regex;

use super::Parser;
use super::itunes_rss_category::ITUNES_RSS_CATEGORY;
use super::itunes_rss_owner::ITUNES_RSS_OWNER;
use super::podlove_chapter::PODLOVE_CHAPTER;
use super::rss::{self, rss_entry};
use super::rss_image::RSS_IMAGE;
use crate::model::{Format, Itunes};
use crate::rules::{Class, Field, attr, collection, object, text};
use crate::sax::Parsed;

pub(super) struct ItunesRss;

static ITUNES_RSS: Class = Class::new(&[
    text("description", Field::Description),
    object("image", Field::Image, &RSS_IMAGE),
    text("language", Field::Language),
    text("link", Field::Url),
    text("title", Field::Title),
    attr("itunes:image", Field::ItunesImage, "href"),
    collection("itunes:category", Field::Nested, &ITUNES_RSS_CATEGORY),
    collection("itunes:owner", Field::Nested, &ITUNES_RSS_OWNER),
    collection("item", Field::Entries, &ITUNES_RSS_ITEM),
]);

/// An RSS item whose `enclosure` is the episode, not an image.
static ITUNES_RSS_ITEM: Class = Class::new(rss_entry!(
    [];
    text("itunes:author", Field::ItunesAuthor),
    text("itunes:block", Field::Other),
    text("itunes:duration", Field::ItunesDuration),
    text("itunes:explicit", Field::ItunesExplicit),
    text("itunes:keywords", Field::Other),
    text("itunes:subtitle", Field::ItunesSubtitle),
    attr("itunes:image", Field::ItunesImage, "href"),
    text("itunes:isClosedCaptioned", Field::Other),
    text("itunes:order", Field::Other),
    text("itunes:season", Field::ItunesSeason),
    text("itunes:episode", Field::ItunesEpisode),
    text("itunes:title", Field::ItunesTitle),
    text("itunes:episodeType", Field::ItunesEpisodeType),
    text("itunes:summary", Field::ItunesSummary),
    attr("enclosure", Field::EnclosureLength, "length"),
    attr("enclosure", Field::EnclosureType, "type"),
    attr("enclosure", Field::EnclosureUrl, "url"),
    collection("psc:chapter", Field::Nested, PODLOVE_CHAPTER),
));

impl Parser for ItunesRss {
    /// Declares the iTunes namespace outside any CDATA section.
    fn able_to_parse(&self, text: &str) -> bool {
        static CDATA: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?s)<!\[CDATA\[.*?\]\]>").unwrap());
        static NAMESPACE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"(?i)xmlns:itunes\s?=\s?["']http://www\.itunes\.com/dtds/podcast-1\.0\.dtd["']"#,
            )
            .unwrap()
        });
        NAMESPACE.is_match(&CDATA.replace_all(text, ""))
    }

    fn format(&self) -> Format {
        Format::Rss
    }

    fn feed(&self) -> &'static Class {
        &ITUNES_RSS
    }

    fn entry_id<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        rss::entry_id(entry)
    }

    fn entry_url<'a>(&self, entry: &Parsed<'a>) -> Option<&'a str> {
        rss::entry_url(entry)
    }

    fn itunes(&self, entry: &Parsed) -> Option<Itunes> {
        let field = |field| entry.single(field).map(String::from);
        Some(Itunes {
            duration: field(Field::ItunesDuration),
            episode: field(Field::ItunesEpisode),
            season: field(Field::ItunesSeason),
            episode_type: field(Field::ItunesEpisodeType),
            explicit: field(Field::ItunesExplicit),
            image: field(Field::ItunesImage),
            subtitle: field(Field::ItunesSubtitle),
            summary: field(Field::ItunesSummary),
            title: field(Field::ItunesTitle),
        })
    }
}
