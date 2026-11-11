//! An XML feed as the model: a parser's raw values, normalized.
//!
//! The entry's `id`, `title` and `url` are trimmed, its author falls back
//! to the iTunes author, and its categories are trimmed with empty ones
//! dropped. Everything else is as parsed.

use crate::model::{Enclosure, Entry, Feed};
use crate::parser::Parser;
use crate::rules::Field;
use crate::sax::{self, Parsed};

pub(crate) fn parse(text: &str, parser: &dyn Parser, declared_encoding: bool) -> Feed {
    let handler = sax::parse(text, parser, declared_encoding);
    feed(parser, &handler.root())
}

fn feed(parser: &dyn Parser, feed: &Parsed) -> Feed {
    Feed {
        format: parser.format(),
        title: owned(feed.single(Field::Title)),
        description: owned(feed.single(Field::Description)),
        site_url: owned(parser.site_url(feed)),
        self_url: owned(parser.self_url(feed)),
        hubs: feed
            .multi(Field::Hubs)
            .flatten()
            .map(String::from)
            .collect(),
        language: owned(feed.single(Field::Language)),
        icon: owned(feed.single(Field::Icon)),
        image_url: owned(
            feed.object(Field::Image)
                .and_then(|image| image.single(Field::Url)),
        ),
        itunes_image: owned(feed.single(Field::ItunesImage)),
        youtube_channel_id: owned(feed.single(Field::YoutubeChannelId)),
        entries: feed
            .entries()
            .map(|entry| self::entry(parser, &entry))
            .collect(),
    }
}

fn entry(parser: &dyn Parser, entry: &Parsed) -> Entry {
    let author = entry
        .single(Field::Author)
        .or_else(|| entry.single(Field::ItunesAuthor));
    let enclosure = entry.single(Field::EnclosureUrl).map(|url| Enclosure {
        url: url.to_string(),
        mime_type: owned(entry.single(Field::EnclosureType)),
        length: owned(entry.single(Field::EnclosureLength)),
    });
    Entry {
        id: parser.entry_id(entry).map(trimmed),
        title: parser.entry_title(entry).as_deref().map(trimmed),
        url: parser.entry_url(entry).map(trimmed),
        author: owned(author),
        content: owned(entry.single(Field::Content)),
        summary: owned(entry.single(Field::Summary)),
        published: entry.published().or(entry.updated()),
        updated: entry.updated(),
        categories: entry
            .multi(Field::Categories)
            .map(|category| category.unwrap_or_default().trim())
            .filter(|category| !category.is_empty())
            .map(String::from)
            .collect(),
        image: owned(entry.single(Field::Image)),
        enclosure,
        enclosures: entry.enclosures().to_vec(),
        media: entry.media().cloned(),
        itunes: parser.itunes(entry),
        youtube: parser.youtube(entry),
    }
}

fn trimmed(value: &str) -> String {
    value.trim().to_string()
}

fn owned(value: Option<&str>) -> Option<String> {
    value.map(String::from)
}
