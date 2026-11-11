//! Subscribing to and editing feeds.

use mariner_feeds::finder::FoundFeed;
use rusqlite::{Transaction, params};
use sha1::{Digest, Sha1};
use url::Url;

use super::articles;
use crate::database::Database;
use crate::model::Feed;
use crate::{Result, queries, time};

/// Title of a feed that has none.
const UNTITLED: &str = "(No title)";

/// Stores the feed `found` describes, with its tags and current entries.
pub(super) async fn subscribe(
    database: &Database,
    found: FoundFeed,
    title: Option<&str>,
    tag_names: Vec<String>,
) -> Result<Feed> {
    let title = title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .or(found
            .feed
            .title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty()))
        .unwrap_or(UNTITLED)
        .to_string();
    let feed = database
        .write(move |connection| {
            let FoundFeed { url, feed } = found;
            let id = feed_id(&url);
            let site_url = feed.site_url.clone().or_else(|| url_from_host(&url));
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO feeds (id, subscription_id, title, feed_url, site_url, itunes_image_url)
                 VALUES (?1, ?1, ?2, ?3, ?4, ?5)",
                params![id, title, url, site_url, feed.itunes_image],
            )?;
            replace_taggings(&transaction, &id, &tag_names)?;
            articles::insert(&transaction, &id, &url, &feed.entries, time::now())?;
            let stored = queries::feed(&transaction, &id)?;
            transaction.commit()?;
            Ok(stored.expect("just inserted"))
        })
        .await?;
    Ok(feed)
}

/// Renames the feed and replaces its tags.
pub(super) async fn update(
    database: &Database,
    id: &str,
    title: &str,
    tag_names: Vec<String>,
) -> Result<Feed> {
    let (id, title) = (id.to_string(), title.to_string());
    let feed = database
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "UPDATE feeds SET title = ?2 WHERE id = ?1",
                params![id, title],
            )?;
            replace_taggings(&transaction, &id, &tag_names)?;
            let stored = queries::feed(&transaction, &id)?;
            transaction.commit()?;
            stored.ok_or(rusqlite::Error::QueryReturnedNoRows)
        })
        .await?;
    Ok(feed)
}

/// A local feed's id: the SHA-1 of its URL, so re-adding a feed gives it
/// the same id.
fn feed_id(url: &str) -> String {
    Sha1::digest(url.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `http://<host>`, for a feed that names no site.
fn url_from_host(url: &str) -> Option<String> {
    let host = Url::parse(url).ok()?.host_str()?.to_string();
    Some(format!("http://{host}"))
}

fn replace_taggings(
    transaction: &Transaction,
    feed_id: &str,
    tag_names: &[String],
) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM taggings WHERE feed_id = ?1", [feed_id])?;
    let mut tagging = transaction.prepare_cached(
        "INSERT OR IGNORE INTO taggings (id, feed_id, name) VALUES (?1 || ':' || ?2, ?1, ?2)",
    )?;
    let mut tag = transaction.prepare_cached("INSERT OR IGNORE INTO tags (name) VALUES (?1)")?;
    for name in tag_names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
    {
        tagging.execute(params![feed_id, name])?;
        tag.execute([name])?;
    }
    Ok(())
}
