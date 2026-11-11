//! Storing parsed entries as articles.

use std::collections::HashSet;

use mariner_feeds::Entry;
use rusqlite::{Transaction, params};

use crate::sql::account::articles;

/// Inserts each new entry of the feed subscribed at `feed_url` as an
/// unread article, with its enclosures. Entries already stored are left
/// alone. Returns how many were inserted.
pub(super) fn insert(
    transaction: &Transaction,
    feed_id: &str,
    feed_url: &str,
    entries: &[Entry],
    now: i64,
) -> rusqlite::Result<usize> {
    let mut article = transaction.prepare_cached(articles::INSERT_ARTICLE)?;
    let mut status = transaction.prepare_cached(articles::INSERT_UNREAD_STATUS)?;
    let mut enclosure = transaction.prepare_cached(articles::INSERT_ENCLOSURE)?;

    let mut seen = HashSet::new();
    let mut inserted = 0;
    for entry in entries {
        let id = entry.public_id(feed_url);
        if !seen.insert(id.clone()) {
            continue;
        }
        let added = article.execute(params![
            id,
            feed_id,
            entry.title,
            entry.author,
            content(entry),
            entry.url,
            entry.summary,
            image_url(entry),
            enclosure_type(entry),
            published_at(entry, now),
            now,
        ])?;
        if added == 0 {
            continue;
        }
        inserted += 1;
        status.execute(params![id, now])?;
        let itunes = entry.itunes.as_ref();
        let duration = itunes
            .and_then(|itunes| itunes.duration.as_deref())
            .and_then(duration_seconds);
        let itunes_image = itunes.and_then(|itunes| itunes.image.as_deref());
        for item in &entry.enclosures {
            enclosure.execute(params![
                id,
                item.url,
                item.mime_type.as_deref().unwrap_or_default(),
                duration,
                itunes_image,
            ])?;
        }
    }
    Ok(inserted)
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
