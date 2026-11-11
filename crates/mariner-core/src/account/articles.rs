//! Storing a feed's entries as articles: new ones are added, changed ones
//! updated, and ones that left the feed eventually deleted.
//!
//! This follows NetNewsWire's feed-based retention:
//! - Updating an article leaves its status alone, so a read article stays
//!   read.
//! - Items published more than [`STALE_AFTER`] ago start out read.
//! - Deleting an article keeps its status as a tombstone, so it isn't added
//!   again while the feed still lists it. Tombstones go after
//!   [`TOMBSTONE_LIFETIME`], by which time the article has left the feed.

use std::collections::{HashMap, HashSet};

use mariner_feeds::Entry;
use rusqlite::{Transaction, params};

use crate::Result;
use crate::database::Database;
use crate::sql::account::{articles, statuses};

const DAY: i64 = 24 * 60 * 60;
/// Items published longer ago than this start out read (about 6 months).
pub(super) const STALE_AFTER: i64 = 183 * DAY;
/// Articles that left their feed are kept this long after arriving.
pub(super) const KEEP_AFTER_LEAVING: i64 = 30 * DAY;
/// How long a local account keeps a deleted article's status.
pub(super) const TOMBSTONE_LIFETIME: i64 = 30 * DAY;

/// What storing a feed's entries changed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Changes {
    pub new: usize,
    pub updated: usize,
    pub deleted: usize,
}

/// Stores the entries of the feed subscribed at `feed_url`:
/// - new entries become articles, unread unless stale, with their
///   enclosures (skipping tombstoned ones);
/// - stored articles whose fields changed are updated;
/// - unstarred articles no longer in the feed are deleted once they arrived
///   [`KEEP_AFTER_LEAVING`] ago, or, when read, before `cutoff`.
///
/// A feed with no entries deletes nothing, in case it's only briefly empty.
pub(super) fn store(
    transaction: &Transaction,
    feed_id: &str,
    feed_url: &str,
    entries: &[Entry],
    now: i64,
    cutoff: Option<i64>,
) -> rusqlite::Result<Changes> {
    let mut changes = Changes::default();
    let stored = stored_fields(transaction, feed_id)?;
    let mut update = transaction.prepare_cached(articles::UPDATE_ARTICLE)?;
    let mut ids = HashSet::new();
    for entry in entries {
        let id = entry.public_id(feed_url);
        if !ids.insert(id.clone()) {
            continue;
        }
        let fields = Fields::from(entry);
        match stored.get(&id) {
            Some(stored) if *stored == fields => {}
            Some(_) => {
                update.execute(params![
                    id,
                    fields.title,
                    fields.author,
                    fields.content_html,
                    fields.url,
                    fields.summary,
                    fields.image_url,
                    fields.enclosure_type,
                ])?;
                changes.updated += 1;
            }
            None => {
                if insert(transaction, &id, feed_id, entry, fields, now)? {
                    changes.new += 1;
                }
            }
        }
    }
    if !ids.is_empty() {
        let listed = serde_json::to_string(&ids).expect("strings serialize");
        changes.deleted = transaction
            .prepare_cached(articles::DELETE_ARTICLES_GONE_FROM_FEED)?
            .execute(params![feed_id, listed, now - KEEP_AFTER_LEAVING, cutoff])?;
    }
    Ok(changes)
}

/// Deletes statuses whose article is gone, unstarred and unchanged since
/// `before`.
pub(super) async fn delete_old_tombstones(database: &Database, before: i64) -> Result<()> {
    database
        .write(move |connection| {
            connection
                .prepare_cached(statuses::DELETE_OLD_TOMBSTONES)?
                .execute([before])
        })
        .await?;
    Ok(())
}

/// The fields of an article that an update compares and writes.
#[derive(Debug, PartialEq, Eq)]
struct Fields {
    title: Option<String>,
    author: Option<String>,
    content_html: Option<String>,
    url: Option<String>,
    summary: Option<String>,
    image_url: Option<String>,
    enclosure_type: Option<String>,
}

impl From<&Entry> for Fields {
    fn from(entry: &Entry) -> Self {
        Fields {
            title: entry.title.clone(),
            author: entry.author.clone(),
            content_html: content(entry).map(String::from),
            url: entry.url.clone(),
            summary: entry.summary.clone(),
            image_url: image_url(entry).map(String::from),
            enclosure_type: enclosure_type(entry).map(String::from),
        }
    }
}

fn stored_fields(
    transaction: &Transaction,
    feed_id: &str,
) -> rusqlite::Result<HashMap<String, Fields>> {
    transaction
        .prepare_cached(articles::SELECT_FEED_ARTICLES)?
        .query_map([feed_id], |row| {
            Ok((
                row.get(0)?,
                Fields {
                    title: row.get(1)?,
                    author: row.get(2)?,
                    content_html: row.get(3)?,
                    url: row.get(4)?,
                    summary: row.get(5)?,
                    image_url: row.get(6)?,
                    enclosure_type: row.get(7)?,
                },
            ))
        })?
        .collect()
}

/// Inserts a new article with its status and enclosures, unless it's
/// tombstoned. Returns whether it was inserted.
fn insert(
    transaction: &Transaction,
    id: &str,
    feed_id: &str,
    entry: &Entry,
    fields: Fields,
    now: i64,
) -> rusqlite::Result<bool> {
    let published_at = published_at(entry, now);
    let added = transaction
        .prepare_cached(articles::INSERT_ARTICLE)?
        .execute(params![
            id,
            feed_id,
            fields.title,
            fields.author,
            fields.content_html,
            fields.url,
            fields.summary,
            fields.image_url,
            fields.enclosure_type,
            published_at,
            now,
        ])?;
    if added == 0 {
        return Ok(false);
    }
    let read = published_at < now - STALE_AFTER;
    transaction
        .prepare_cached(articles::INSERT_STATUS)?
        .execute(params![id, read, now])?;
    let itunes = entry.itunes.as_ref();
    let duration = itunes
        .and_then(|itunes| itunes.duration.as_deref())
        .and_then(duration_seconds);
    let itunes_image = itunes.and_then(|itunes| itunes.image.as_deref());
    let mut enclosure = transaction.prepare_cached(articles::INSERT_ENCLOSURE)?;
    for item in &entry.enclosures {
        enclosure.execute(params![
            id,
            item.url,
            item.mime_type.as_deref().unwrap_or_default(),
            duration,
            itunes_image,
        ])?;
    }
    Ok(true)
}

/// The content, or else the summary, ignoring blank ones.
fn content(entry: &Entry) -> Option<&str> {
    [entry.content.as_deref(), entry.summary.as_deref()]
        .into_iter()
        .flatten()
        .find(|text| !text.trim().is_empty())
}

fn image_url(entry: &Entry) -> Option<&str> {
    entry.image.as_deref().or_else(|| {
        entry
            .media
            .as_ref()
            .and_then(|media| media.thumbnails.first())
            .map(|thumbnail| thumbnail.url.as_str())
    })
}

fn enclosure_type(entry: &Entry) -> Option<&str> {
    entry
        .enclosure
        .as_ref()
        .and_then(|enclosure| enclosure.mime_type.as_deref())
        .or_else(|| {
            entry
                .enclosures
                .iter()
                .find_map(|enclosure| enclosure.mime_type.as_deref())
        })
}

/// The entry's date, or `now` when it's missing, in the future or not
/// after 1970.
fn published_at(entry: &Entry, now: i64) -> i64 {
    entry
        .published
        .map(|published| published.timestamp())
        .filter(|published| *published > 0 && *published <= now)
        .unwrap_or(now)
}

/// `1:02:03`, `62:03` or `3723` as seconds.
fn duration_seconds(duration: &str) -> Option<i64> {
    duration.trim().split(':').try_fold(0i64, |total, part| {
        Some(total * 60 + part.trim().parse::<i64>().ok()?)
    })
}

#[cfg(test)]
mod tests {
    use super::duration_seconds;

    #[test]
    fn reads_itunes_durations() {
        assert_eq!(duration_seconds("1:02:03"), Some(3723));
        assert_eq!(duration_seconds("62:03"), Some(3723));
        assert_eq!(duration_seconds("3723"), Some(3723));
        assert_eq!(duration_seconds("about an hour"), None);
    }
}
