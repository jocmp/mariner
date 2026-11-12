//! The bindings as an app uses them, with an `HttpClient` written in Rust
//! standing in for URLSession or OkHttp.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures_executor::block_on;
use mariner_ffi::{
    Account, AddFeedResult, ArticleFilter, ArticleScope, HttpClient, HttpError, HttpHeader,
    HttpRequest, HttpResponse, MarinerError,
};

const FEED: &str = r#"<rss version="2.0"><channel><title>Example</title>
<link>https://example.com/</link>
<item><title>First</title><guid>1</guid><link>https://example.com/1</link></item>
</channel></rss>"#;

/// Answers from a table of responses by URL, recording each request. An
/// unknown URL can't connect.
#[derive(Default)]
struct Stub {
    responses: Mutex<HashMap<String, HttpResponse>>,
    requests: Mutex<Vec<String>>,
}

impl Stub {
    fn with(self, url: &str, status: u16, headers: &[(&str, &str)], body: &str) -> Self {
        let response = HttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| HttpHeader {
                    name: name.to_string(),
                    value: value.to_string(),
                })
                .collect(),
            body: body.as_bytes().to_vec(),
        };
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), response);
        self
    }
}

#[async_trait::async_trait]
impl HttpClient for Stub {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        self.requests.lock().unwrap().push(request.url.clone());
        let response = self.responses.lock().unwrap().get(&request.url).cloned();
        response.ok_or_else(|| HttpError::Connect {
            reason: format!("no route to {}", request.url),
        })
    }
}

fn open(stub: Stub) -> (tempfile::TempDir, Arc<Account>, Arc<Stub>) {
    let dir = tempfile::tempdir().unwrap();
    let stub = Arc::new(stub);
    let path = dir.path().join("local.db").to_string_lossy().into_owned();
    let account = block_on(Account::local(path, stub.clone())).unwrap();
    (dir, account, stub)
}

#[test]
fn adds_a_feed_through_a_redirect_and_lists_its_articles() {
    let stub = Stub::default()
        .with(
            "http://example.com/feed",
            301,
            &[("Location", "https://example.com/feed")],
            "",
        )
        .with(
            "https://example.com/feed",
            200,
            &[("Content-Type", "application/rss+xml")],
            FEED,
        );
    let (_dir, account, stub) = open(stub);

    let result = block_on(account.add_feed("http://example.com/feed".into(), None, None)).unwrap();
    let AddFeedResult::Added { feed } = result else {
        panic!("expected a feed, got {result:?}");
    };
    assert_eq!(feed.title, "Example");
    assert_eq!(
        stub.requests.lock().unwrap()[..2],
        ["http://example.com/feed", "https://example.com/feed"],
        "the app sends each hop; core follows the redirect"
    );

    let scope = ArticleScope::Feed {
        id: feed.id.clone(),
    };
    let articles = block_on(account.articles(ArticleFilter::Unread, scope.clone(), 10, 0)).unwrap();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title.as_deref(), Some("First"));

    block_on(account.mark_all_read(scope)).unwrap();
    assert!(block_on(account.unread_counts()).unwrap().is_empty());
}

#[test]
fn reports_the_apps_network_errors() {
    let (_dir, account, _) = open(Stub::default());
    let error = block_on(account.add_feed("https://offline.example/".into(), None, None))
        .expect_err("nothing answers");
    assert!(
        matches!(&error, MarinerError::Network { reason } if reason.contains("no route")),
        "{error:?}"
    );
}

#[test]
fn reports_a_failed_page_as_its_status() {
    let stub = Stub::default().with("https://example.com/gone", 404, &[], "");
    let (_dir, account, _) = open(stub);
    let error = block_on(account.full_content("https://example.com/gone".into()))
        .expect_err("a 404 has no article");
    assert!(
        matches!(&error, MarinerError::Status { status: 404, .. }),
        "{error:?}"
    );
}
