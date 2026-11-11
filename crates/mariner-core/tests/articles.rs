//! Listing articles and marking them all read.

use common::{Network, local_account};
use futures_executor::block_on;
use mariner_core::{Account, AddFeedResult, ArticleFilter, ArticleScope, Feed};

mod common;

const NEWS: &str = "https://example.com/news.xml";
const BLOG: &str = "https://example.com/blog.xml";

/// Items dated a day apart, newest first, starting `days_ago` days back.
fn rss(title: &str, items: &[&str], days_ago: i64) -> String {
    let items: String = items
        .iter()
        .enumerate()
        .map(|(index, guid)| {
            let date = chrono::Utc::now() - chrono::Duration::days(days_ago + index as i64);
            format!(
                "<item><title>{guid}</title><guid>{guid}</guid><pubDate>{}</pubDate></item>",
                date.to_rfc2822()
            )
        })
        .collect();
    format!(r#"<rss version="2.0"><channel><title>{title}</title>{items}</channel></rss>"#)
}

async fn two_feeds() -> (tempfile::TempDir, Account<Network>, Feed) {
    let network = Network::default()
        .with(NEWS, &rss("News", &["n1", "n2", "n3"], 0))
        .with(BLOG, &rss("Blog", &["b1", "b2"], 10));
    let (dir, account) = local_account(&network).await;
    let AddFeedResult::Added(news) = account.add_feed(NEWS, None, None).await.unwrap() else {
        panic!("expected a feed");
    };
    account.add_feed(BLOG, None, None).await.unwrap();
    (dir, account, news)
}

#[test]
fn lists_articles_newest_first_in_pages() {
    block_on(async {
        let (_dir, account, _) = two_feeds().await;
        let titles = |articles: Vec<mariner_core::Article>| -> Vec<String> {
            articles.into_iter().filter_map(|a| a.title).collect()
        };
        let all = account
            .articles(ArticleFilter::All, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        assert_eq!(titles(all), ["n1", "n2", "n3", "b1", "b2"]);

        let page = account
            .articles(ArticleFilter::All, &ArticleScope::All, 2, 2)
            .await
            .unwrap();
        assert_eq!(titles(page), ["n3", "b1"]);
    });
}

#[test]
fn filters_by_state_and_feed() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        let all = account
            .articles(ArticleFilter::All, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        let id = |title: &str| {
            all.iter()
                .find(|a| a.title.as_deref() == Some(title))
                .unwrap()
                .id
                .clone()
        };
        account.mark_read(&[id("n1"), id("b1")]).await.unwrap();
        account.add_star(&[id("n2")]).await.unwrap();

        let titles = |articles: Vec<mariner_core::Article>| -> Vec<String> {
            articles.into_iter().filter_map(|a| a.title).collect()
        };
        let unread = account
            .articles(ArticleFilter::Unread, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        assert_eq!(titles(unread), ["n2", "n3", "b2"]);
        let starred = account
            .articles(ArticleFilter::Starred, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        assert_eq!(titles(starred), ["n2"]);
        let news_unread = account
            .articles(
                ArticleFilter::Unread,
                &ArticleScope::Feed(news.id.clone()),
                10,
                0,
            )
            .await
            .unwrap();
        assert_eq!(titles(news_unread), ["n2", "n3"]);

        let listed = account
            .articles(ArticleFilter::All, &ArticleScope::All, 1, 0)
            .await
            .unwrap();
        assert!(listed[0].read && !listed[0].starred);
    });
}

#[test]
fn marks_all_read_in_one_feed_or_every_feed() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        account
            .mark_all_read(&ArticleScope::Feed(news.id.clone()))
            .await
            .unwrap();
        let unread = account
            .articles(ArticleFilter::Unread, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        let titles: Vec<_> = unread.into_iter().filter_map(|a| a.title).collect();
        assert_eq!(titles, ["b1", "b2"]);

        account.mark_all_read(&ArticleScope::All).await.unwrap();
        assert!(
            account
                .articles(ArticleFilter::Unread, &ArticleScope::All, 10, 0)
                .await
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn reads_one_article_with_its_content() {
    block_on(async {
        let (_dir, account, _) = two_feeds().await;
        let first = &account
            .articles(ArticleFilter::All, &ArticleScope::All, 1, 0)
            .await
            .unwrap()[0];
        let content = account.article(&first.id).await.unwrap().unwrap();
        assert_eq!(&content.article, first);
        assert!(account.article("missing").await.unwrap().is_none());
    });
}

#[test]
fn counts_unread_articles_per_feed() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        let blog = account
            .feeds()
            .await
            .unwrap()
            .into_iter()
            .find(|feed| feed.id != news.id)
            .unwrap();
        account
            .mark_all_read(&ArticleScope::Feed(blog.id.clone()))
            .await
            .unwrap();
        let counts = account.unread_counts().await.unwrap();
        assert_eq!(counts.get(&news.id), Some(&3));
        assert_eq!(counts.get(&blog.id), None);
    });
}

#[test]
fn lists_and_marks_read_by_tag() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        account
            .update_feed(&news, &news.title, &["Daily".to_string()])
            .await
            .unwrap();
        let daily = ArticleScope::Tag("Daily".into());
        let titles: Vec<String> = account
            .articles(ArticleFilter::Unread, &daily, 10, 0)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|article| article.title)
            .collect();
        assert_eq!(titles, ["n1", "n2", "n3"]);

        account.mark_all_read(&daily).await.unwrap();
        let unread = account
            .articles(ArticleFilter::Unread, &ArticleScope::All, 10, 0)
            .await
            .unwrap();
        assert_eq!(unread.len(), 2, "only the untagged blog's are left");
    });
}

#[test]
fn remembers_whether_a_feed_opens_in_full_text() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        assert!(!news.sticky_full_content);
        account
            .set_sticky_full_content(&news.id, true)
            .await
            .unwrap();
        let feeds = account.feeds().await.unwrap();
        let sticky: Vec<&str> = feeds
            .iter()
            .filter(|feed| feed.sticky_full_content)
            .map(|feed| feed.title.as_str())
            .collect();
        assert_eq!(sticky, ["News"]);
    });
}

#[test]
fn remembers_whether_a_folder_is_expanded() {
    block_on(async {
        let (_dir, account, news) = two_feeds().await;
        account
            .update_feed(&news, &news.title, &["Daily".to_string()])
            .await
            .unwrap();
        let tags = account.tags().await.unwrap();
        assert_eq!(tags.len(), 1);
        assert!(!tags[0].expanded, "folders start collapsed");

        account.set_tag_expanded("Daily", true).await.unwrap();
        assert!(account.tags().await.unwrap()[0].expanded);
    });
}
