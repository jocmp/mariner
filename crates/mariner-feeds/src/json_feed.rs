//! JSON Feed (https://jsonfeed.org).

use serde_json::{Map, Value};

use crate::model::{Enclosure, Entry, Feed, Format};
use crate::{Error, date};

pub(crate) fn is_json_feed(text: &str) -> bool {
    text.contains("https://jsonfeed.org/version/")
        || text.contains(r"https:\/\/jsonfeed.org\/version\/")
}

pub(crate) fn parse(text: &str) -> Result<Feed, Error> {
    let Ok(Value::Object(feed)) = serde_json::from_str::<Value>(text) else {
        return Err(Error::NotAFeed);
    };
    let feed_author = author_name(feed.get("author"));
    let entries = match feed.get("items") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_object)
            .map(|item| entry(item, feed_author.as_deref()))
            .collect(),
        _ => Vec::new(),
    };
    Ok(Feed {
        format: Format::Json,
        title: string(&feed, "title"),
        description: string(&feed, "description"),
        site_url: string(&feed, "home_page_url"),
        self_url: string(&feed, "feed_url"),
        hubs: match feed.get("hubs") {
            Some(Value::Array(hubs)) => hubs
                .iter()
                .filter_map(|hub| hub.get("url").and_then(Value::as_str).map(String::from))
                .collect(),
            _ => Vec::new(),
        },
        language: string(&feed, "language"),
        icon: string(&feed, "icon"),
        image_url: None,
        itunes_image: None,
        entries,
    })
}

/// An item. The author falls back to the feed's, the content prefers
/// `content_html` to `content_text`, and the URL falls back to
/// `external_url`.
fn entry(item: &Map<String, Value>, feed_author: Option<&str>) -> Entry {
    let attachment = match item.get("attachments") {
        Some(Value::Array(attachments)) => attachments.first().and_then(Value::as_object),
        _ => None,
    };
    Entry {
        id: item
            .get("id")
            .filter(|id| !id.is_null())
            .map(|id| match id {
                Value::String(id) => id.trim().to_string(),
                other => other.to_string(),
            }),
        title: string(item, "title").map(|title| title.trim().to_string()),
        url: string(item, "url")
            .or_else(|| string(item, "external_url"))
            .map(|url| url.trim().to_string()),
        author: author_name(item.get("author")).or_else(|| feed_author.map(String::from)),
        content: string(item, "content_html").or_else(|| string(item, "content_text")),
        summary: string(item, "summary"),
        published: string(item, "date_published")
            .as_deref()
            .and_then(date::parse),
        updated: string(item, "date_modified")
            .as_deref()
            .and_then(date::parse),
        categories: match item.get("tags") {
            Some(Value::Array(tags)) => tags
                .iter()
                .map(|tag| match tag {
                    Value::String(tag) => tag.trim().to_string(),
                    other => other.to_string(),
                })
                .filter(|tag| !tag.is_empty())
                .collect(),
            _ => Vec::new(),
        },
        image: string(item, "image"),
        enclosure: attachment.and_then(|attachment| {
            Some(Enclosure {
                url: string(attachment, "url")?,
                mime_type: string(attachment, "mime_type"),
                length: attachment
                    .get("size_in_bytes")
                    .filter(|size| !size.is_null())
                    .map(|size| size.to_string()),
            })
        }),
        itunes: None,
    }
}

fn author_name(author: Option<&Value>) -> Option<String> {
    author?
        .get("name")
        .and_then(Value::as_str)
        .map(String::from)
}

fn string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(String::from)
}
