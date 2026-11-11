//! Subscribing to, editing and removing feeds and tags.

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

/// Renames a tag on every feed. A feed that already has the new tag keeps
/// one tagging.
pub(super) async fn rename_tag(database: &Database, old_name: &str, new_name: &str) -> Result<()> {
    let (old_name, new_name) = (old_name.to_string(), new_name.to_string());
    database
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "UPDATE OR IGNORE taggings SET name = ?2, id = feed_id || ':' || ?2
                 WHERE name = ?1",
                [&old_name, &new_name],
            )?;
            transaction.execute("DELETE FROM taggings WHERE name = ?1", [&old_name])?;
            transaction.execute(
                "INSERT OR IGNORE INTO tags (name, expanded)
                 SELECT ?2, expanded FROM tags WHERE name = ?1",
                [&old_name, &new_name],
            )?;
            transaction.execute("DELETE FROM tags WHERE name = ?1", [&old_name])?;
            transaction.commit()
        })
        .await?;
    Ok(())
}

/// Deletes the feed with its articles and their statuses. Unsubscribing
/// isn't pruning: without the statuses, re-adding the feed brings its
/// articles back unread.
pub(super) async fn remove(database: &Database, id: &str) -> Result<()> {
    let id = id.to_string();
    database
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "DELETE FROM article_statuses
                 WHERE article_id IN (SELECT id FROM articles WHERE feed_id = ?1)",
                [&id],
            )?;
            transaction.execute("DELETE FROM feeds WHERE id = ?1", [&id])?;
            transaction.commit()
        })
        .await?;
    Ok(())
}

pub(super) async fn remove_tag(database: &Database, name: &str) -> Result<()> {
    let name = name.to_string();
    database
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute("DELETE FROM taggings WHERE name = ?1", [&name])?;
            transaction.execute("DELETE FROM tags WHERE name = ?1", [&name])?;
            transaction.commit()
        })
        .await?;
    Ok(())
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
