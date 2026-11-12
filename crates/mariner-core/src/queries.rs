//! Reading an account's database. The same for every kind of account.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::Result;
use crate::database::Database;
use std::collections::HashMap;

use chrono::DateTime;

use crate::model::{Article, ArticleContent, ArticleFilter, ArticleScope, Feed, Tag};
use crate::sql::account::{article_list, feeds, tags};

/// Every feed, by title (case-insensitively).
pub(crate) async fn feeds(database: &Database) -> Result<Vec<Feed>> {
    Ok(database.read(all_feeds).await?)
}

/// Every tag, by name (case-insensitively).
pub(crate) async fn tags(database: &Database) -> Result<Vec<Tag>> {
    Ok(database
        .read(|connection| {
            connection
                .prepare_cached(tags::SELECT_TAGS)?
                .query_map([], |row| {
                    Ok(Tag {
                        name: row.get(0)?,
                        expanded: row.get(1)?,
                    })
                })?
                .collect()
        })
        .await?)
}

/// A page of articles in `scope`, newest first.
pub(crate) async fn articles(
    database: &Database,
    filter: ArticleFilter,
    scope: &ArticleScope,
    limit: usize,
    offset: usize,
) -> Result<Vec<Article>> {
    let (feed_id, tag) = scope.feed_and_tag();
    let sql = match filter {
        ArticleFilter::All => article_list::SELECT_ARTICLES,
        ArticleFilter::Unread => article_list::SELECT_UNREAD_ARTICLES,
        ArticleFilter::Starred => article_list::SELECT_STARRED_ARTICLES,
    };
    Ok(database
        .read(move |connection| {
            connection
                .prepare_cached(sql)?
                .query_map(
                    params![feed_id, limit as i64, offset as i64, tag],
                    article_row,
                )?
                .collect()
        })
        .await?)
}

/// One article with its content.
pub(crate) async fn article(database: &Database, id: &str) -> Result<Option<ArticleContent>> {
    let id = id.to_string();
    Ok(database
        .read(move |connection| {
            connection
                .prepare_cached(article_list::SELECT_ARTICLE)?
                .query_row(params![id], |row| {
                    Ok(ArticleContent {
                        article: article_row(row)?,
                        content_html: row.get(10)?,
                    })
                })
                .optional()
        })
        .await?)
}

/// Unread articles per feed id; feeds with none are absent.
pub(crate) async fn unread_counts(database: &Database) -> Result<HashMap<String, usize>> {
    Ok(database
        .read(|connection| {
            connection
                .prepare_cached(article_list::SELECT_UNREAD_COUNTS)?
                .query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)))?
                .collect()
        })
        .await?)
}

/// Every unread article's id in `scope`.
pub(crate) async fn unread_article_ids(
    database: &Database,
    scope: &ArticleScope,
) -> Result<Vec<String>> {
    let (feed_id, tag) = scope.feed_and_tag();
    Ok(database
        .read(move |connection| {
            connection
                .prepare_cached(article_list::SELECT_UNREAD_ARTICLE_IDS)?
                .query_map(params![feed_id, tag], |row| row.get(0))?
                .collect()
        })
        .await?)
}

/// The feed subscribed at exactly `url`.
pub(crate) async fn feed_by_url(database: &Database, url: &str) -> Result<Option<Feed>> {
    let url = url.to_string();
    Ok(database
        .read(move |connection| {
            let id: Option<String> = connection
                .prepare_cached(feeds::SELECT_FEED_ID_BY_URL)?
                .query_row(params![url], |row| row.get(0))
                .optional()?;
            id.map(|id| feed(connection, &id))
                .transpose()
                .map(Option::flatten)
        })
        .await?)
}

pub(crate) fn all_feeds(connection: &Connection) -> rusqlite::Result<Vec<Feed>> {
    let mut statement = connection.prepare_cached(feeds::SELECT_FEEDS)?;
    let mut feeds = statement
        .query_map([], feed_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for feed in &mut feeds {
        feed.tag_names = tag_names(connection, &feed.id)?;
    }
    Ok(feeds)
}

pub(crate) fn feed(connection: &Connection, id: &str) -> rusqlite::Result<Option<Feed>> {
    let mut statement = connection.prepare_cached(feeds::SELECT_FEED)?;
    let Some(mut feed) = statement.query_row(params![id], feed_row).optional()? else {
        return Ok(None);
    };
    feed.tag_names = tag_names(connection, id)?;
    Ok(Some(feed))
}

/// A row of `select_feeds` or `select_feed`.
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
        sticky_full_content: row.get(7)?,
    })
}

/// A row of the `article_list` queries.
fn article_row(row: &Row) -> rusqlite::Result<Article> {
    Ok(Article {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        title: row.get(2)?,
        author: row.get(3)?,
        url: row.get(4)?,
        summary: row.get(5)?,
        image_url: row.get(6)?,
        published_at: DateTime::from_timestamp(row.get(7)?, 0).unwrap_or_default(),
        read: row.get(8)?,
        starred: row.get(9)?,
    })
}

pub(crate) fn tag_names(connection: &Connection, feed_id: &str) -> rusqlite::Result<Vec<String>> {
    connection
        .prepare_cached(tags::SELECT_FEED_TAG_NAMES)?
        .query_map(params![feed_id], |row| row.get(0))?
        .collect()
}
