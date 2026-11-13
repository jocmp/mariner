//! Fetches a local account's feeds and stores what's new.
//!
//! Each feed that isn't backing off is fetched with a conditional GET, a
//! few at a time. A feed's new fetch state and new articles are stored
//! together, in one transaction, as soon as its fetch finishes, so a
//! cancelled refresh keeps what it had finished. A failing feed doesn't
//! fail the refresh; a database error does.

use futures_util::stream::{self, StreamExt, TryStreamExt};
use mariner_feeds::Feed as ParsedFeed;
use mariner_feeds::fetch::{Fetch, FetchError, Response};
use rusqlite::{Connection, Row, params};
use sha1::{Digest, Sha1};

use super::crawl_data::{CrawlData, ErrorKind};
use super::redirect_cache::RedirectCache;
use crate::account::{Account, articles};
use crate::sql::account::{crawl, feeds};
use crate::{Result, time};

/// Feeds fetched at once.
const CONCURRENCY: usize = 6;

pub(super) struct LocalAccountRefresher<'a, F> {
    fetcher: &'a F,
}

impl<'a, F: Fetch + Send + Sync> LocalAccountRefresher<'a, F> {
    pub fn new(fetcher: &'a F) -> Self {
        LocalAccountRefresher { fetcher }
    }

    /// Fetches each due feed and stores its new state and entries.
    pub async fn refresh(&self, account: &Account<F>) -> Result<()> {
        let now = time::now();
        let feeds = account
            .database()
            .read(move |connection| due_feeds(connection, now))
            .await?;
        stream::iter(feeds)
            .map(|feed| async move {
                let downloaded = self.download(feed, now).await;
                store(account, downloaded).await
            })
            .buffer_unordered(CONCURRENCY)
            .try_collect::<()>()
            .await
    }

    async fn download(&self, mut feed: DueFeed, now: i64) -> Downloaded {
        feed.crawl.log_download(now);
        let request = feed.crawl.request(&feed.feed_url);
        let parsed = match self.fetcher.fetch(&request).await {
            Ok(response) => read(&mut feed.crawl, response, now),
            Err(error) => {
                let kind = match error {
                    FetchError::Timeout => ErrorKind::Timeout,
                    FetchError::Connect(_) | FetchError::Request(_) => ErrorKind::Network,
                };
                feed.crawl.download_error(kind, None, None, now);
                None
            }
        };
        Downloaded { feed, parsed }
    }
}

/// A feed due for a fetch.
struct DueFeed {
    id: String,
    /// The URL as subscribed, which article ids are based on.
    feed_url: String,
    crawl: CrawlData,
}

struct Downloaded {
    feed: DueFeed,
    /// The feed, when its body changed and parsed.
    parsed: Option<ParsedFeed>,
}

/// Updates the fetch state from a response, returning the feed when the
/// body is new and parses.
fn read(crawl: &mut CrawlData, response: Response, now: i64) -> Option<ParsedFeed> {
    if response.status == 304 {
        crawl.download_success(&response.redirects);
        return None;
    }
    if !response.is_success() {
        crawl.download_error(
            ErrorKind::for_status(response.status),
            Some(response.status),
            response.header("retry-after"),
            now,
        );
        return None;
    }
    let fingerprint = fingerprint(&response.body);
    let unchanged = crawl.fetch_fingerprint.as_deref() == Some(fingerprint.as_str());
    crawl.download_success(&response.redirects);
    crawl.save(
        response.header("etag"),
        response.header("last-modified"),
        fingerprint,
    );
    if unchanged {
        return None;
    }
    match response.feed() {
        Some(feed) => {
            crawl.clear();
            Some(feed)
        }
        None => {
            crawl.download_error(ErrorKind::NotAFeed, Some(response.status), None, now);
            None
        }
    }
}

fn fingerprint(body: &[u8]) -> String {
    Sha1::digest(body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Stores a feed's fetch state and entries in one transaction.
async fn store<F>(account: &Account<F>, downloaded: Downloaded) -> Result<()> {
    account
        .database()
        .write(move |connection| {
            let Downloaded { feed, parsed } = downloaded;
            let transaction = connection.transaction()?;
            update_crawl_data(&transaction, &feed.id, &feed.crawl)?;
            if let Some(parsed) = parsed {
                update_feed(&transaction, &feed.id, &parsed)?;
                articles::store(
                    &transaction,
                    &feed.id,
                    &feed.feed_url,
                    &parsed.entries,
                    time::now(),
                )?;
            }
            transaction.commit()
        })
        .await?;
    Ok(())
}

/// Takes the title and site the feed gives now, keeping the stored ones
/// when it gives none.
fn update_feed(connection: &Connection, id: &str, parsed: &ParsedFeed) -> rusqlite::Result<()> {
    connection
        .prepare_cached(feeds::UPDATE_FEED_FROM_SOURCE)?
        .execute(params![
            id,
            non_blank(parsed.title.as_deref()),
            non_blank(parsed.site_url.as_deref()),
        ])?;
    Ok(())
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

fn due_feeds(connection: &Connection, now: i64) -> rusqlite::Result<Vec<DueFeed>> {
    connection
        .prepare_cached(crawl::SELECT_DUE_FEEDS)?
        .query_map(params![now], due_feed)?
        .collect()
}

/// A row of `select_due_feeds`.
fn due_feed(row: &Row) -> rusqlite::Result<DueFeed> {
    Ok(DueFeed {
        id: row.get(0)?,
        feed_url: row.get(1)?,
        crawl: CrawlData {
            etag: row.get(2)?,
            last_modified: row.get(3)?,
            last_fetched_at: row.get(4)?,
            fetch_fingerprint: row.get(5)?,
            error_count: row.get(6)?,
            failed_at: row.get(7)?,
            last_error_kind: row.get(8)?,
            last_error_status: row.get(9)?,
            retry_after: row.get(10)?,
            redirect: RedirectCache {
                redirected_to: row.get(11)?,
                redirect_count: row.get(12)?,
            },
        },
    })
}

fn update_crawl_data(connection: &Connection, id: &str, crawl: &CrawlData) -> rusqlite::Result<()> {
    connection
        .prepare_cached(crawl::UPDATE_CRAWL_DATA)?
        .execute(params![
            id,
            crawl.etag,
            crawl.last_modified,
            crawl.last_fetched_at,
            crawl.fetch_fingerprint,
            crawl.error_count,
            crawl.failed_at,
            crawl.last_error_kind,
            crawl.last_error_status,
            crawl.retry_after,
            crawl.redirect.redirected_to,
            crawl.redirect.redirect_count,
        ])?;
    Ok(())
}
