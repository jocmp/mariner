//! Reading an account's database. The same for every kind of account.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::Result;
use crate::database::Database;
use crate::model::Feed;

const FEED_COLUMNS: &str =
    "id, subscription_id, title, feed_url, site_url, favicon_url, itunes_image_url";

/// Every feed, by title (case-insensitively).
pub(crate) async fn feeds(database: &Database) -> Result<Vec<Feed>> {
    Ok(database.read(all_feeds).await?)
}

/// The feed subscribed at exactly `url`.
pub(crate) async fn feed_by_url(database: &Database, url: &str) -> Result<Option<Feed>> {
    let url = url.to_string();
    Ok(database
        .read(move |connection| {
            let id: Option<String> = connection
                .prepare_cached("SELECT id FROM feeds WHERE feed_url = ?1")?
                .query_row(params![url], |row| row.get(0))
                .optional()?;
            id.map(|id| feed(connection, &id))
                .transpose()
                .map(Option::flatten)
        })
        .await?)
}

pub(crate) fn all_feeds(connection: &Connection) -> rusqlite::Result<Vec<Feed>> {
    let mut statement = connection.prepare_cached(&format!(
        "SELECT {FEED_COLUMNS} FROM feeds ORDER BY title COLLATE NOCASE, id"
    ))?;
    let mut feeds = statement
        .query_map([], feed_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for feed in &mut feeds {
        feed.tag_names = tag_names(connection, &feed.id)?;
    }
    Ok(feeds)
}

pub(crate) fn feed(connection: &Connection, id: &str) -> rusqlite::Result<Option<Feed>> {
    let mut statement =
        connection.prepare_cached(&format!("SELECT {FEED_COLUMNS} FROM feeds WHERE id = ?1"))?;
    let Some(mut feed) = statement.query_row(params![id], feed_row).optional()? else {
        return Ok(None);
    };
    feed.tag_names = tag_names(connection, id)?;
    Ok(Some(feed))
}

fn feed_row(row: &Row) -> rusqlite::Result<Feed> {
    Ok(Feed {
        id: row.get(0)?,
        subscription_id: row.get(1)?,
        title: row.get(2)?,
        feed_url: row.get(3)?,
        site_url: row.get(4)?,
        favicon_url: row.get(5)?,
        itunes_image_url: row.get(6)?,
        tag_names: Vec::new(),
    })
}

fn tag_names(connection: &Connection, feed_id: &str) -> rusqlite::Result<Vec<String>> {
    connection
        .prepare_cached("SELECT name FROM taggings WHERE feed_id = ?1 ORDER BY name")?
        .query_map(params![feed_id], |row| row.get(0))?
        .collect()
}
