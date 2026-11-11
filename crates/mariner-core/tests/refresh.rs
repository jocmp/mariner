//! Refreshing a local account against a stubbed network.

use common::{Network, local_account};
use futures_executor::block_on;
use mariner_core::Account;

mod common;

const FEED_URL: &str = "https://example.com/feed.xml";

fn rss(items: &[&str]) -> String {
    let items: String = items
        .iter()
        .map(|guid| format!("<item><title>{guid}</title><guid>{guid}</guid></item>"))
        .collect();
    format!(
        r#"<rss version="2.0"><channel><title>Example</title><link>https://example.com/</link>{items}</channel></rss>"#
    )
}

#[test]
fn stores_new_articles() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();

        network.respond(FEED_URL, 200, &[], &rss(&["2", "1"]));
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["1", "2"]);
    });
}

#[test]
fn sends_conditional_headers_and_skips_not_modified() {
    block_on(async {
        let network = Network::default();
        network.respond(
            FEED_URL,
            200,
            &[
                ("ETag", "\"v1\""),
                ("Last-Modified", "Tue, 14 Nov 2023 22:13:20 GMT"),
            ],
            &rss(&["1"]),
        );
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        account.refresh(None).await.unwrap();

        network.clear_requests();
        network.respond(FEED_URL, 304, &[], "");
        account.refresh(None).await.unwrap();
        let request = &network.requests()[0];
        assert_eq!(
            request.headers,
            [
                ("If-None-Match".to_string(), "\"v1\"".to_string()),
                (
                    "If-Modified-Since".to_string(),
                    "Tue, 14 Nov 2023 22:13:20 GMT".to_string()
                ),
            ]
        );
        assert_eq!(crawl(&account).await.error_count, 0);
    });
}

#[test]
fn skips_an_unchanged_body() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        account.refresh(None).await.unwrap();
        let fingerprint = crawl(&account).await.fingerprint;
        assert!(fingerprint.is_some());

        // A pruned article would come back if the body were parsed again.
        delete_articles(&account).await;
        account.refresh(None).await.unwrap();
        assert!(titles(&account).await.is_empty());
        assert_eq!(crawl(&account).await.fingerprint, fingerprint);
    });
}

#[test]
fn backs_off_a_failing_feed() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();

        network.respond(FEED_URL, 503, &[("Retry-After", "60")], "");
        account.refresh(None).await.unwrap();
        let state = crawl(&account).await;
        assert_eq!(state.error_count, 1);
        assert_eq!(state.last_error_kind.as_deref(), Some("server_error"));
        assert_eq!(state.last_error_status, Some(503));
        assert!(state.retry_after.unwrap() > state.failed_at.unwrap() + 4000);

        network.clear_requests();
        account.refresh(None).await.unwrap();
        assert!(network.requests().is_empty(), "skipped while backing off");
    });
}

#[test]
fn records_network_errors_and_feeds_that_stop_being_feeds() {
    block_on(async {
        let other = "https://example.com/other.xml";
        let network = Network::default()
            .with(FEED_URL, &rss(&["1"]))
            .with(other, &rss(&["a"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        account.add_feed(other, None, None).await.unwrap();

        network.unreachable(FEED_URL);
        network.respond(other, 200, &[], "<html><body>Moved</body></html>");
        account.refresh(None).await.unwrap();

        let kinds = error_kinds(&account).await;
        assert_eq!(
            kinds,
            [
                (FEED_URL.to_string(), Some("network".to_string())),
                (other.to_string(), Some("not_a_feed".to_string())),
            ]
        );
    });
}

#[test]
fn trusts_a_permanent_redirect_after_five_sightings() {
    block_on(async {
        let moved = "https://feeds.example.com/feed.xml";
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        let feed = match account.add_feed(FEED_URL, None, None).await.unwrap() {
            mariner_core::AddFeedResult::Added(feed) => feed,
            other => panic!("{other:?}"),
        };

        network.redirect(FEED_URL, 301, moved);
        for refresh in 1..=5 {
            network.respond(moved, 200, &[], &rss(&["1", &format!("r{refresh}")]));
            account.refresh(None).await.unwrap();
        }
        network.clear_requests();
        account.refresh(None).await.unwrap();
        assert_eq!(network.requested_urls(), [moved]);

        let feeds = account.feeds().await.unwrap();
        assert_eq!(
            feeds[0].feed_url, FEED_URL,
            "the subscribed URL never changes"
        );
        assert_eq!(feeds[0].id, feed.id);
        assert_eq!(titles(&account).await, ["1", "r1", "r2", "r3", "r4", "r5"]);
    });
}

#[test]
fn updates_changed_articles_and_keeps_their_status() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        let id = article_ids(&account).await["1"].clone();
        account.mark_read(std::slice::from_ref(&id)).await.unwrap();

        let edited = rss(&["1"]).replace("<title>1</title>", "<title>1, edited</title>");
        network.respond(FEED_URL, 200, &[], &edited);
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["1, edited"]);
        assert_eq!(read_ids(&account).await, [id]);
    });
}

#[test]
fn deletes_articles_that_left_the_feed_after_30_days() {
    block_on(async {
        let network =
            Network::default().with(FEED_URL, &rss(&["old", "recent", "starred", "listed"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        let ids = article_ids(&account).await;
        account.add_star(&[ids["starred"].clone()]).await.unwrap();
        arrived_days_ago(&account, &["old", "starred", "listed"], 31).await;

        network.respond(FEED_URL, 200, &[], &rss(&["listed", "new"]));
        account.refresh(None).await.unwrap();
        assert_eq!(
            titles(&account).await,
            ["listed", "new", "recent", "starred"]
        );
    });
}

#[test]
fn an_empty_feed_deletes_nothing() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        arrived_days_ago(&account, &["1"], 31).await;

        network.respond(FEED_URL, 200, &[], &rss(&[]));
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["1"]);
    });
}

#[test]
fn the_cutoff_deletes_read_articles_once_they_leave_the_feed() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["read", "unread", "listed"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        let ids = article_ids(&account).await;
        account
            .mark_read(&[ids["read"].clone(), ids["listed"].clone()])
            .await
            .unwrap();

        network.respond(FEED_URL, 200, &[], &rss(&["listed"]));
        let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
        account.refresh(Some(tomorrow)).await.unwrap();
        assert_eq!(titles(&account).await, ["listed", "unread"]);
    });
}

#[test]
fn tombstones_keep_deleted_articles_out_until_they_expire() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["1", "2"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        arrived_days_ago(&account, &["1", "2"], 31).await;
        network.respond(FEED_URL, 200, &[], &rss(&["3"]));
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["3"]);
        assert_eq!(
            status_count(&account).await,
            3,
            "deleted articles leave tombstones"
        );

        // Tombstones keep the deleted articles out if the feed lists them again.
        network.respond(FEED_URL, 200, &[], &rss(&["1", "3"]));
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["3"]);

        // A month on, unchanged tombstones are forgotten.
        status_changed_days_ago(&account, 31).await;
        network.respond(FEED_URL, 200, &[], &rss(&["3", "4"]));
        account.refresh(None).await.unwrap();
        assert_eq!(status_count(&account).await, 2, "the statuses of 3 and 4");
    });
}

#[test]
fn stale_items_start_read() {
    block_on(async {
        let feed = r#"<rss version="2.0"><channel><title>Example</title>
            <item><title>stale</title><guid>stale</guid><pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate></item>
            <item><title>fresh</title><guid>fresh</guid></item>
            </channel></rss>"#;
        let network = Network::default().with(FEED_URL, feed);
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        let ids = article_ids(&account).await;
        assert_eq!(read_ids(&account).await, [ids["stale"].clone()]);
    });
}

#[derive(Debug)]
struct Crawl {
    error_count: i64,
    failed_at: Option<i64>,
    last_error_kind: Option<String>,
    last_error_status: Option<i64>,
    retry_after: Option<i64>,
    fingerprint: Option<String>,
}

async fn crawl(account: &Account<Network>) -> Crawl {
    account
        .database()
        .read(|connection| {
            connection.query_row(
                "SELECT error_count, failed_at, last_error_kind, last_error_status,
                        retry_after, fetch_fingerprint
                 FROM feeds",
                [],
                |row| {
                    Ok(Crawl {
                        error_count: row.get(0)?,
                        failed_at: row.get(1)?,
                        last_error_kind: row.get(2)?,
                        last_error_status: row.get(3)?,
                        retry_after: row.get(4)?,
                        fingerprint: row.get(5)?,
                    })
                },
            )
        })
        .await
        .unwrap()
}

async fn error_kinds(account: &Account<Network>) -> Vec<(String, Option<String>)> {
    account
        .database()
        .read(|connection| {
            connection
                .prepare("SELECT feed_url, last_error_kind FROM feeds ORDER BY feed_url")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect()
        })
        .await
        .unwrap()
}

async fn titles(account: &Account<Network>) -> Vec<String> {
    account
        .database()
        .read(|connection| {
            connection
                .prepare("SELECT title FROM articles ORDER BY title")?
                .query_map([], |row| row.get(0))?
                .collect()
        })
        .await
        .unwrap()
}

async fn article_ids(account: &Account<Network>) -> std::collections::HashMap<String, String> {
    account
        .database()
        .read(|connection| {
            connection
                .prepare("SELECT title, id FROM articles")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect()
        })
        .await
        .unwrap()
}

async fn delete_articles(account: &Account<Network>) {
    account
        .database()
        .write(|connection| {
            connection.execute_batch("DELETE FROM article_statuses; DELETE FROM articles;")
        })
        .await
        .unwrap();
}

async fn read_ids(account: &Account<Network>) -> Vec<String> {
    account
        .database()
        .read(|connection| {
            connection
                .prepare("SELECT article_id FROM article_statuses WHERE read = 1")?
                .query_map([], |row| row.get(0))?
                .collect()
        })
        .await
        .unwrap()
}

async fn status_count(account: &Account<Network>) -> i64 {
    account
        .database()
        .read(|connection| {
            connection.query_row("SELECT count(*) FROM article_statuses", [], |row| {
                row.get(0)
            })
        })
        .await
        .unwrap()
}

/// Moves the arrival of the articles with these titles back in time.
async fn arrived_days_ago(account: &Account<Network>, titles: &[&str], days: i64) {
    let titles: Vec<String> = titles.iter().map(|title| title.to_string()).collect();
    account
        .database()
        .write(move |connection| {
            for title in &titles {
                connection.execute(
                    "UPDATE articles SET created_at = created_at - ?2 * 86400 WHERE title = ?1",
                    rusqlite::params![title, days],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

/// Moves every status's last change back in time.
async fn status_changed_days_ago(account: &Account<Network>, days: i64) {
    account
        .database()
        .write(move |connection| {
            connection.execute(
                "UPDATE article_statuses SET updated_at = updated_at - ?1 * 86400",
                [days],
            )
        })
        .await
        .unwrap();
}
