//! The parsed-feed model. Its serde JSON is also the parity format.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Serialize, Serializer};
use sha1::{Digest, Sha1};

/// A parsed feed document, before anything is stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Feed {
    pub format: Format,
    pub title: Option<String>,
    pub description: Option<String>,
    /// The site the feed belongs to.
    pub site_url: Option<String>,
    /// The URL the feed says it lives at.
    pub self_url: Option<String>,
    /// WebSub hubs.
    pub hubs: Vec<String>,
    pub language: Option<String>,
    pub icon: Option<String>,
    pub image_url: Option<String>,
    pub itunes_image: Option<String>,
    pub youtube_channel_id: Option<String>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Rss,
    Atom,
    Json,
}

/// One item in a feed. Strings are as written in the document, except
/// `id`, `title` and `url`, which are trimmed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Entry {
    /// The entry's own identifier (RSS `guid`, Atom `id`, JSON Feed `id`).
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub author: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    /// The earliest publication date given, or else `updated`.
    #[serde(serialize_with = "serialize_time")]
    pub published: Option<DateTime<Utc>>,
    /// The latest modification date given.
    #[serde(serialize_with = "serialize_time")]
    pub updated: Option<DateTime<Utc>>,
    pub categories: Vec<String>,
    pub image: Option<String>,
    /// The enclosure of an entry in a podcast (iTunes) feed or a JSON Feed
    /// attachment. Other feeds' enclosures are only in `enclosures`.
    pub enclosure: Option<Enclosure>,
    /// Every enclosure the entry lists: RSS `<enclosure>` elements, Atom
    /// `rel="enclosure"` links and JSON Feed attachments.
    pub enclosures: Vec<Enclosure>,
    /// Media RSS (`media:*`), when the entry has any.
    pub media: Option<Media>,
    /// Present for entries of podcast (iTunes) feeds.
    pub itunes: Option<Itunes>,
    /// Present for entries of YouTube feeds.
    pub youtube: Option<Youtube>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Enclosure {
    pub url: String,
    #[serde(rename = "type")]
    pub mime_type: Option<String>,
    pub length: Option<String>,
}

/// Media RSS for an entry (https://www.rssboard.org/media-rss).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Media {
    pub title: Option<String>,
    pub description: Option<String>,
    pub contents: Vec<MediaContent>,
    pub thumbnails: Vec<MediaThumbnail>,
    pub star_average: Option<String>,
    pub star_count: Option<String>,
    pub views: Option<String>,
}

/// A `media:content`. Numbers are as written.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaContent {
    pub url: String,
    #[serde(rename = "type")]
    pub mime_type: Option<String>,
    /// `image`, `audio`, `video`, `document` or `executable`.
    pub medium: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub file_size: Option<String>,
    pub duration: Option<String>,
    pub title: Option<String>,
    /// Often an image's alt text.
    pub description: Option<String>,
    /// Which `media:group` of the entry this is in, counting from 0.
    /// Contents in one group are versions of the same media (different
    /// sizes or formats); contents in no group are separate media.
    pub group: Option<usize>,
    /// Marked `isDefault="true"`: the version to use when there's no other
    /// reason to pick one.
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaThumbnail {
    pub url: String,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Youtube {
    pub video_id: Option<String>,
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Itunes {
    pub duration: Option<String>,
    pub episode: Option<String>,
    pub season: Option<String>,
    pub episode_type: Option<String>,
    pub explicit: Option<String>,
    pub image: Option<String>,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub title: Option<String>,
}

impl Entry {
    /// A stable article ID for this entry in the feed subscribed at
    /// `feed_url` (the URL as subscribed, not where it redirects):
    /// `sha1_hex(feed_url + id)`, or, for an entry without an id,
    /// `sha1_hex(feed_url + url + published + title)`, skipping missing
    /// parts. `published` is ISO 8601 in UTC, to the second.
    pub fn public_id(&self, feed_url: &str) -> String {
        let mut input = String::from(feed_url);
        match &self.id {
            Some(id) => input.push_str(id),
            None => {
                input.push_str(self.url.as_deref().unwrap_or_default());
                if let Some(published) = self.published {
                    input.push_str(&iso8601(published));
                }
                input.push_str(self.title.as_deref().unwrap_or_default());
            }
        }
        Sha1::digest(input.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

fn serialize_time<S: Serializer>(
    time: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match time {
        Some(time) => serializer.serialize_str(&iso8601(*time)),
        None => serializer.serialize_none(),
    }
}

fn iso8601(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Secs, true)
}
