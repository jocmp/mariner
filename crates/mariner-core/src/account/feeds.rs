//! Subscribing to, editing and removing feeds and tags.

use mariner_feeds::finder::FoundFeed;
use rusqlite::{Transaction, params};
use sha1::{Digest, Sha1};
use url::Url;

use super::articles;
use crate::database::Database;
use crate::model::Feed;
use crate::sql::account::{feeds, tags};
use crate::{Result, queries, time};

/// Title of a feed that has none.
const UNTITLED: &str = "(No title)";

/// Stores the feed `found` describes, with its current entries.
pub(super) async fn subscribe(
    database: &Database,
    found: FoundFeed,
    title: Option<&str>,
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
                feeds::INSERT_FEED,
                params![id, title, url, site_url, feed.itunes_image],
            )?;
            articles::store(&transaction, &id, &url, &feed.entries, time::now(), None)?;
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
            transaction.execute(feeds::UPDATE_FEED_TITLE, params![id, title])?;
            replace_taggings(&transaction, &id, &tag_names)?;
            let stored = queries::feed(&transaction, &id)?;
            transaction.commit()?;
            stored.ok_or(rusqlite::Error::QueryReturnedNoRows)
        })
        .await?;
    Ok(feed)
}

pub(super) async fn set_sticky_full_content(
    database: &Database,
    id: &str,
    enabled: bool,
) -> Result<()> {
    let id = id.to_string();
    database
        .write(move |connection| {
            connection
                .prepare_cached(feeds::UPDATE_FEED_STICKY_FULL_CONTENT)?
                .execute(params![id, enabled])
        })
        .await?;
    Ok(())
}

pub(super) async fn set_tag_expanded(
    database: &Database,
    name: &str,
    expanded: bool,
) -> Result<()> {
    let name = name.to_string();
    database
        .write(move |connection| {
            connection
                .prepare_cached(tags::UPDATE_TAG_EXPANDED)?
                .execute(params![name, expanded])
        })
        .await?;
    Ok(())
}

/// Renames a tag on every feed. A feed that already has the new tag keeps
/// one tagging.
pub(super) async fn rename_tag(database: &Database, old_name: &str, new_name: &str) -> Result<()> {
    let (old_name, new_name) = (old_name.to_string(), new_name.to_string());
    database
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute(tags::RENAME_TAGGINGS, [&old_name, &new_name])?;
            transaction.execute(tags::DELETE_TAGGINGS_NAMED, [&old_name])?;
            transaction.execute(tags::COPY_TAG, [&old_name, &new_name])?;
            transaction.execute(tags::DELETE_TAG, [&old_name])?;
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
            transaction.execute(feeds::DELETE_FEED_STATUSES, [&id])?;
            transaction.execute(feeds::DELETE_FEED, [&id])?;
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
            transaction.execute(tags::DELETE_TAGGINGS_NAMED, [&name])?;
            transaction.execute(tags::DELETE_TAG, [&name])?;
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
    transaction.execute(tags::DELETE_FEED_TAGGINGS, [feed_id])?;
    let mut tagging = transaction.prepare_cached(tags::INSERT_TAGGING)?;
    let mut tag = transaction.prepare_cached(tags::INSERT_TAG)?;
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
