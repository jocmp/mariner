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
fn prunes_old_read_articles_and_keeps_them_out() {
    block_on(async {
        let network = Network::default().with(FEED_URL, &rss(&["read", "starred", "unread"]));
        let (_dir, account) = local_account(&network).await;
        account.add_feed(FEED_URL, None, None).await.unwrap();
        let ids = article_ids(&account).await;
        account
            .mark_read(&[ids["read"].clone(), ids["starred"].clone()])
            .await
            .unwrap();
        account.add_star(&[ids["starred"].clone()]).await.unwrap();

        let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
        account.refresh(Some(tomorrow)).await.unwrap();
        assert_eq!(titles(&account).await, ["starred", "unread"]);

        // The feed still lists the pruned article; its status keeps it out.
        network.respond(
            FEED_URL,
            200,
            &[],
            &rss(&["read", "starred", "unread", "new"]),
        );
        account.refresh(None).await.unwrap();
        assert_eq!(titles(&account).await, ["new", "starred", "unread"]);
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
