//! A local account against stubbed fetches, driven without an async runtime.

use common::{Network, local_account};
use futures_executor::block_on;
use mariner_core::{Account, Error};

mod common;

const FEED_URL: &str = "https://example.com/feed.xml";

const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <channel>
    <title>Example</title>
    <link>https://example.com/</link>
    <itunes:image href="https://example.com/cover.jpg"/>
    <item>
      <title>First</title>
      <guid>https://example.com/1</guid>
      <pubDate>Sat, 05 Oct 2024 10:11:12 GMT</pubDate>
      <description>Summary one</description>
      <itunes:duration>1:02:03</itunes:duration>
      <enclosure url="https://example.com/1.mp3" type="audio/mpeg" length="100"/>
    </item>
    <item>
      <title>Second, dated in the future</title>
      <guid>https://example.com/2</guid>
      <pubDate>Sat, 05 Oct 2099 10:11:12 GMT</pubDate>
    </item>
    <item>
      <title>Duplicate of the first</title>
      <guid>https://example.com/1</guid>
    </item>
  </channel>
</rss>"#;

#[test]
fn adds_a_feed_with_its_articles() {
    block_on(async {
        let network = Network::default().with(FEED_URL, RSS);
        let (_dir, account) = local_account(&network).await;
        let feed = account.subscribe(FEED_URL, None).await.unwrap();

        assert_eq!(feed.title, "Example");
        assert_eq!(feed.feed_url, FEED_URL);
        assert_eq!(feed.site_url.as_deref(), Some("https://example.com/"));
        assert_eq!(
            feed.itunes_image_url.as_deref(),
            Some("https://example.com/cover.jpg")
        );
        assert_eq!(account.feeds().await.unwrap(), std::slice::from_ref(&feed));
        assert_eq!(network.requested_urls(), [FEED_URL]);

        let articles = articles(&account).await;
        let first = mariner_feeds::Entry {
            id: Some("https://example.com/1".into()),
            ..Default::default()
        };
        assert_eq!(articles.len(), 2, "duplicates are stored once");
        let stored = articles
            .iter()
            .find(|a| a.id == first.public_id(FEED_URL))
            .unwrap();
        assert_eq!(stored.title.as_deref(), Some("First"));
        assert_eq!(stored.published_at, 1728123072);
        assert_eq!(stored.content_html.as_deref(), Some("Summary one"));
        assert_eq!(stored.enclosure_type.as_deref(), Some("audio/mpeg"));
        assert!(
            stored.read,
            "published over 6 months ago, so it starts read"
        );

        let future = articles
            .iter()
            .find(|a| a.title.as_deref() == Some("Second, dated in the future"))
            .unwrap();
        assert!(
            future.published_at < 4_000_000_000,
            "a future date becomes now"
        );

        let enclosure = enclosures(&account).await;
        assert_eq!(
            enclosure,
            [(
                "https://example.com/1.mp3".to_string(),
                "audio/mpeg".to_string(),
                Some(3723)
            )]
        );
    });
}

#[test]
fn uses_the_given_title_then_folders() {
    block_on(async {
        let (_dir, account) = local_account(&Network::default().with(FEED_URL, RSS)).await;
        let feed = account.subscribe(FEED_URL, Some(" Mine ")).await.unwrap();
        assert_eq!(feed.title, "Mine");
        assert!(feed.tag_names.is_empty());

        let tags = vec!["News".to_string(), " ".to_string(), "Audio".to_string()];
        let feed = account
            .update_feed(&feed, &feed.title, &tags)
            .await
            .unwrap();
        assert_eq!(feed.tag_names, ["Audio", "News"]);
    });
}

#[test]
fn finds_an_existing_feed_without_fetching() {
    block_on(async {
        let network = Network::default().with(FEED_URL, RSS);
        let (_dir, account) = local_account(&network).await;
        let first = account.subscribe(FEED_URL, None).await.unwrap();
        let again = account.subscribe(FEED_URL, Some("Other")).await.unwrap();
        assert_eq!(first, again);
        assert_eq!(network.requested_urls(), [FEED_URL]);
    });
}

#[test]
fn finds_every_feed_a_page_has_without_subscribing() {
    block_on(async {
        let page = r#"<link rel="alternate" type="application/rss+xml" href="/feed.xml">
                      <link rel="alternate" type="application/rss+xml" href="/comments.xml">"#;
        let stub = Network::default()
            .with("https://example.com/", page)
            .with(FEED_URL, RSS)
            .with(
                "https://example.com/comments.xml",
                &RSS.replace("Example", "Comments"),
            );
        let (_dir, account) = local_account(&stub).await;
        let choices = account.find_feeds("https://example.com/").await.unwrap();
        let choices: Vec<_> = choices
            .iter()
            .map(|c| (c.url.as_str(), c.title.as_deref()))
            .collect();
        assert_eq!(
            choices,
            [
                (FEED_URL, Some("Example")),
                ("https://example.com/comments.xml", Some("Comments"))
            ]
        );
        assert!(account.feeds().await.unwrap().is_empty());

        let error = account
            .subscribe("https://example.com/", None)
            .await
            .expect_err("a page with several feeds is no one feed");
        assert!(matches!(error, Error::NoFeed { .. }), "{error:?}");

        let comments = account
            .subscribe("https://example.com/comments.xml", None)
            .await
            .unwrap();
        assert_eq!(comments.title, "Comments");
    });
}

#[test]
fn reports_a_page_without_feeds() {
    block_on(async {
        let stub = Network::default().with("https://example.com/", "<html><body>Hi</body></html>");
        let (_dir, account) = local_account(&stub).await;
        let found = account.find_feeds("https://example.com/").await.unwrap();
        assert!(found.is_empty());
    });
}

#[test]
fn marks_articles_read_and_starred() {
    block_on(async {
        let (_dir, account) = local_account(&Network::default().with(FEED_URL, RSS)).await;
        account.subscribe(FEED_URL, None).await.unwrap();
        let ids: Vec<String> = articles(&account).await.into_iter().map(|a| a.id).collect();

        account.mark_read(&ids).await.unwrap();
        account.add_star(&ids[..1]).await.unwrap();
        account.mark_unread(&ids[1..]).await.unwrap();
        account.mark_read(&["missing".to_string()]).await.unwrap();

        let mut states: Vec<_> = articles(&account)
            .await
            .into_iter()
            .map(|a| (a.id, a.read, a.starred))
            .collect();
        states.sort();
        let mut expected = vec![(ids[0].clone(), true, true), (ids[1].clone(), false, false)];
        expected.sort();
        assert_eq!(states, expected);

        account.remove_star(&ids[..1]).await.unwrap();
        assert!(articles(&account).await.iter().all(|a| !a.starred));
        assert_eq!(
            status_count(&account).await,
            2,
            "no status for unknown articles"
        );
    });
}

#[test]
fn updates_a_feed() {
    block_on(async {
        let (_dir, account) = local_account(&Network::default().with(FEED_URL, RSS)).await;
        let feed = account.subscribe(FEED_URL, None).await.unwrap();
        let feed = account
            .update_feed(&feed, &feed.title, &["Old".to_string()])
            .await
            .unwrap();
        let updated = account
            .update_feed(&feed, "Renamed", &["B".to_string(), "A".to_string()])
            .await
            .unwrap();
        assert_eq!(updated.title, "Renamed");
        assert_eq!(updated.tag_names, ["A", "B"]);
        assert_eq!(account.feeds().await.unwrap(), [updated]);
    });
}

#[test]
fn renames_and_removes_tags() {
    block_on(async {
        let other = "https://example.com/other.xml";
        let stub = Network::default().with(FEED_URL, RSS).with(other, RSS);
        let (_dir, account) = local_account(&stub).await;
        let first = account.subscribe(FEED_URL, None).await.unwrap();
        account
            .update_feed(
                &first,
                &first.title,
                &["Old".to_string(), "New".to_string()],
            )
            .await
            .unwrap();
        let second = account.subscribe(other, None).await.unwrap();
        account
            .update_feed(&second, &second.title, &["Old".to_string()])
            .await
            .unwrap();

        account.update_tag("Old", "New").await.unwrap();
        let tags: Vec<_> = account
            .feeds()
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.tag_names)
            .collect();
        assert_eq!(tags, [vec!["New".to_string()], vec!["New".to_string()]]);

        account.remove_tag("New").await.unwrap();
        assert!(
            account
                .feeds()
                .await
                .unwrap()
                .iter()
                .all(|f| f.tag_names.is_empty())
        );
    });
}

#[test]
fn removing_a_feed_removes_its_articles_and_statuses() {
    block_on(async {
        let (_dir, account) = local_account(&Network::default().with(FEED_URL, RSS)).await;
        let feed = account.subscribe(FEED_URL, None).await.unwrap();
        let ids: Vec<String> = articles(&account).await.into_iter().map(|a| a.id).collect();
        account.add_star(&ids).await.unwrap();

        account.remove_feed(&feed).await.unwrap();
        assert!(account.feeds().await.unwrap().is_empty());
        assert!(articles(&account).await.is_empty());
        assert_eq!(status_count(&account).await, 0);

        let again = account.subscribe(FEED_URL, None).await.unwrap();
        assert_eq!(again.id, feed.id);
        assert_eq!(
            articles(&account).await.len(),
            2,
            "articles come back unread"
        );
    });
}

#[derive(Debug)]
struct Article {
    id: String,
    title: Option<String>,
    content_html: Option<String>,
    enclosure_type: Option<String>,
    published_at: i64,
    read: bool,
    starred: bool,
}

async fn articles(account: &Account<Network>) -> Vec<Article> {
    account
        .database()
        .read(|connection| {
            let mut statement = connection.prepare(
                "SELECT a.id, a.title, a.content_html, a.enclosure_type, a.published_at,
                        COALESCE(s.read, 0), COALESCE(s.starred, 0)
                 FROM articles a LEFT JOIN article_statuses s ON s.article_id = a.id",
            )?;
            statement
                .query_map([], |row| {
                    Ok(Article {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        content_html: row.get(2)?,
                        enclosure_type: row.get(3)?,
                        published_at: row.get(4)?,
                        read: row.get(5)?,
                        starred: row.get(6)?,
                    })
                })?
                .collect()
        })
        .await
        .unwrap()
}

async fn enclosures(account: &Account<Network>) -> Vec<(String, String, Option<i64>)> {
    account
        .database()
        .read(|connection| {
            connection
                .prepare("SELECT url, type, itunes_duration_seconds FROM enclosures")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
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

#[test]
fn removing_a_folders_last_feed_removes_the_folder() {
    block_on(async {
        let other = "https://example.com/other.xml";
        let stub = Network::default().with(FEED_URL, RSS).with(other, RSS);
        let (_dir, account) = local_account(&stub).await;
        let first = account.subscribe(FEED_URL, None).await.unwrap();
        let first = account
            .update_feed(
                &first,
                &first.title,
                &["Solo".to_string(), "Shared".to_string()],
            )
            .await
            .unwrap();
        let second = account.subscribe(other, None).await.unwrap();
        account
            .update_feed(&second, &second.title, &["Shared".to_string()])
            .await
            .unwrap();

        account.remove_feed(&first).await.unwrap();
        let tags: Vec<String> = account
            .tags()
            .await
            .unwrap()
            .into_iter()
            .map(|tag| tag.name)
            .collect();
        assert_eq!(tags, ["Shared"], "Solo had no other feed");
    });
}
