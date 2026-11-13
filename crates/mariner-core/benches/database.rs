//! Database benchmarks: a refresh's worth of new articles, the whole
//! article list, marking everything read, and importing and exporting a
//! subscription list. Driven by a minimal executor,
//! like the app's would be, with no network.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use futures_executor::block_on;
use mariner_core::{Account, ArticleFilter, ArticleScope, Database, Kind};
use mariner_feeds::fetch::{Fetch, FetchError, Request, Response};

const FEEDS: usize = 50;
const ITEMS_PER_FEED: usize = 20;

criterion_group!(benches, refresh, article_list, mark_all_read, opml);
criterion_main!(benches);

/// 50 subscribed feeds, each serving 20 items it hasn't served before.
fn refresh(c: &mut Criterion) {
    let mut group = c.benchmark_group("refresh");
    group.sample_size(20);
    group.bench_function("50 feeds x 20 new articles", |b| {
        b.iter_batched(
            || {
                let network = Network::default();
                let (dir, account) = block_on(subscribed(&network));
                network.serve_generation(1);
                (dir, account)
            },
            |(_dir, account)| block_on(account.refresh()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// 50,000 articles, a fifth of them read, listed whole as the apps do.
fn article_list(c: &mut Criterion) {
    let (_dir, account) = block_on(seeded(50_000, 5));
    let mut group = c.benchmark_group("article list of 50,000");
    group.sample_size(20);
    group.bench_function("every article", |b| {
        b.iter(|| {
            block_on(account.articles(ArticleFilter::All, &ArticleScope::All, None)).unwrap()
        });
    });
    group.bench_function("every unread article", |b| {
        b.iter(|| {
            block_on(account.articles(ArticleFilter::Unread, &ArticleScope::All, None)).unwrap()
        });
    });
    group.finish();
}

/// 5,000 unread articles.
fn mark_all_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("mark all read");
    group.sample_size(20);
    group.bench_function("5,000 unread", |b| {
        b.iter_batched(
            || block_on(seeded(5_000, 0)),
            |(_dir, account)| block_on(account.mark_all_read(&ArticleScope::All)).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// 1,000 feeds in 20 folders, imported into an empty account and exported.
fn opml(c: &mut Criterion) {
    let list = opml_list(1_000, 20);
    let mut group = c.benchmark_group("opml");
    group.sample_size(20);
    group.bench_function("import 1,000 feeds", |b| {
        b.iter_batched(
            || block_on(empty_account(Network::default())),
            |(_dir, account)| block_on(account.import_opml(list.as_bytes())).unwrap(),
            BatchSize::PerIteration,
        );
    });
    let (_dir, account) = block_on(empty_account(Network::default()));
    block_on(account.import_opml(list.as_bytes())).unwrap();
    group.bench_function("export 1,000 feeds", |b| {
        b.iter(|| block_on(account.export_opml()).unwrap());
    });
    group.finish();
}

/// An OPML document of `feeds` feeds spread over `folders` folders.
fn opml_list(feeds: usize, folders: usize) -> String {
    let mut body = String::new();
    for folder in 0..folders {
        body.push_str(&format!("<outline text=\"Folder {folder}\">"));
        for feed in (folder..feeds).step_by(folders) {
            body.push_str(&format!(
                "<outline text=\"Feed {feed}\" xmlUrl=\"https://example.com/{feed}.xml\" htmlUrl=\"https://example.com/\"/>"
            ));
        }
        body.push_str("</outline>");
    }
    format!("<opml version=\"2.0\"><body>{body}</body></opml>")
}

/// An account subscribed to every feed the network serves.
async fn subscribed(network: &Network) -> (tempfile::TempDir, Account<Network>) {
    network.serve_generation(0);
    let (dir, account) = empty_account(network.clone()).await;
    for feed in 0..FEEDS {
        account.subscribe(&feed_url(feed), None).await.unwrap();
    }
    (dir, account)
}

/// An account with `count` articles in one feed, every `read_every`th one
/// read (0 for none).
async fn seeded(count: usize, read_every: usize) -> (tempfile::TempDir, Account<Network>) {
    let (dir, account) = empty_account(Network::default()).await;
    account
        .database()
        .write(move |connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO feeds (id, subscription_id, title, feed_url) VALUES ('f', 'f', 'Feed', 'https://example.com/f.xml')",
                [],
            )?;
            {
                let mut article = transaction.prepare(
                    "INSERT INTO articles (id, feed_id, title, summary, published_at, created_at)
                     VALUES (?1, 'f', ?2, 'A short summary of the article.', ?3, ?3)",
                )?;
                let mut status = transaction.prepare(
                    "INSERT INTO article_statuses (article_id, read, updated_at) VALUES (?1, ?2, ?3)",
                )?;
                for index in 0..count {
                    let id = format!("{index:040x}");
                    let time = 1_700_000_000 + index as i64 * 60;
                    let read = read_every > 0 && index % read_every == 0;
                    article.execute(rusqlite::params![id, format!("Article {index}"), time])?;
                    status.execute(rusqlite::params![id, read, time])?;
                }
            }
            transaction.commit()
        })
        .await
        .unwrap();
    // As a refresh would leave it.
    account.database().optimize().await.unwrap();
    (dir, account)
}

async fn empty_account(network: Network) -> (tempfile::TempDir, Account<Network>) {
    let dir = tempfile::tempdir().unwrap();
    let database = Database::open(dir.path().join("bench.db"), Kind::Account)
        .await
        .unwrap();
    (dir, Account::local(database, network))
}

fn feed_url(feed: usize) -> String {
    format!("https://example.com/{feed}.xml")
}

/// Serves each feed's current body.
#[derive(Clone, Default)]
struct Network {
    bodies: Arc<Mutex<HashMap<String, String>>>,
}

impl Network {
    /// Gives every feed `ITEMS_PER_FEED` items unique to `generation`.
    fn serve_generation(&self, generation: usize) {
        let mut bodies = self.bodies.lock().unwrap();
        for feed in 0..FEEDS {
            let items: String = (0..ITEMS_PER_FEED)
                .map(|item| {
                    format!(
                        "<item><title>Item {generation}-{item}</title><guid>{feed}-{generation}-{item}</guid>\
                         <link>https://example.com/{feed}/{generation}/{item}</link>\
                         <description><![CDATA[<p>Paragraph one of item {item}.</p><p>Paragraph two.</p>]]></description>\
                         <pubDate>Mon, 09 Nov 2026 10:00:00 GMT</pubDate></item>"
                    )
                })
                .collect();
            let body = format!(
                r#"<rss version="2.0"><channel><title>Feed {feed}</title><link>https://example.com/</link>{items}</channel></rss>"#
            );
            bodies.insert(feed_url(feed), body);
        }
    }
}

impl Fetch for Network {
    async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
        let body = self.bodies.lock().unwrap().get(&request.url).cloned();
        Ok(Response {
            url: request.url.clone(),
            status: if body.is_some() { 200 } else { 404 },
            body: body.unwrap_or_default().into_bytes(),
            ..Response::default()
        })
    }
}
