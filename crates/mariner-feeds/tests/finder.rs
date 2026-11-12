//! FeedFinder and each of its sources, against stubbed responses.

use std::collections::HashMap;
use std::sync::Mutex;

use mariner_feeds::fetch::{Fetch, FetchError, Request, Response};
use mariner_feeds::finder::{
    BodyLinks, FindError, Guess, KnownPattern, MetaLinks, Source, Xml, find, find_options,
};

const ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom</title>
  <link rel="alternate" type="text/html" href="https://example.com/"/>
  <entry><title>One</title><id>urn:one</id></entry>
</feed>"#;

const JSON_FEED: &str = r#"{"version": "https://jsonfeed.org/version/1.1", "title": "JSON",
  "items": [{"id": "1", "url": "https://example.com/1"}]}"#;

const HTML: &str = "<!doctype html><html><head><title>Page</title></head><body></body></html>";

mod xml {
    use super::*;

    #[tokio::test]
    async fn finds_an_atom_feed() {
        let stub = Stub::default().with("https://example.com/atom.xml", ATOM);
        let response = stub.download("https://example.com/atom.xml").await;
        assert_eq!(
            urls(Xml::find(&stub, &response).await),
            ["https://example.com/atom.xml"]
        );
    }

    #[tokio::test]
    async fn finds_a_json_feed() {
        let stub = Stub::default().with("https://example.com/feed.json", JSON_FEED);
        let response = stub.download("https://example.com/feed.json").await;
        assert_eq!(
            urls(Xml::find(&stub, &response).await),
            ["https://example.com/feed.json"]
        );
    }

    #[tokio::test]
    async fn finds_nothing_in_html() {
        let stub = Stub::default().with("https://example.com", HTML);
        let response = stub.download("https://example.com").await;
        assert!(Xml::find(&stub, &response).await.is_empty());
    }

    #[tokio::test]
    async fn ignores_pages_that_only_mention_feed_markup() {
        let stub = Stub::default().with(
            "https://example.com",
            r#"<html><body><script>const tag = "<rss>";</script></body></html>"#,
        );
        let response = stub.download("https://example.com").await;
        assert!(Xml::find(&stub, &response).await.is_empty());
    }
}

mod meta_links {
    use super::*;

    #[tokio::test]
    async fn finds_atom_links() {
        let stub = Stub::default()
            .with(
                "https://example.com",
                r#"<link rel="alternate" type="application/atom+xml" href="https://example2.com/comments/feed/"/>"#,
            )
            .with("https://example2.com/comments/feed/", ATOM);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(MetaLinks::find(&stub, &response).await),
            ["https://example2.com/comments/feed/"]
        );
    }

    #[tokio::test]
    async fn finds_rss_links() {
        let stub = Stub::default()
            .with(
                "https://example.com",
                r#"<link rel="alternate" type="application/atom+xml" href="/comments/feed/"/>"#,
            )
            .with("https://example.com/comments/feed/", ATOM);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(MetaLinks::find(&stub, &response).await),
            ["https://example.com/comments/feed/"]
        );
    }

    #[tokio::test]
    async fn finds_json_links() {
        let stub = Stub::default()
            .with(
                "https://example.com",
                r#"<link rel="alternate" type="application/json" href="/feeds/json/"/>"#,
            )
            .with("https://example.com/feeds/json/", JSON_FEED);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(MetaLinks::find(&stub, &response).await),
            ["https://example.com/feeds/json/"]
        );
    }
}

mod body_links {
    use super::*;

    #[tokio::test]
    async fn finds_candidate_links() {
        let mut stub = Stub::default();
        let mut markup = String::new();
        for kind in ["feed", "xml", "rss", "atom"] {
            stub = stub.with(&format!("https://example.com/{kind}"), ATOM);
            markup.push_str(&format!(r#"<a href="/{kind}">RSS</a>"#));
        }
        let stub = stub.with("https://example.com", &markup);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(BodyLinks::find(&stub, &response).await),
            [
                "https://example.com/feed",
                "https://example.com/xml",
                "https://example.com/rss",
                "https://example.com/atom",
            ]
        );
    }

    #[tokio::test]
    async fn skips_html_links() {
        let stub = Stub::default()
            .with("https://example.com", r#"<a href="/rss">RSS</a>"#)
            .with("https://example.com/rss", HTML);
        let response = stub.download("https://example.com").await;
        assert!(BodyLinks::find(&stub, &response).await.is_empty());
    }
}

mod known_pattern {
    use super::*;

    #[tokio::test]
    async fn recognizes_known_patterns() {
        let cases = [
            (
                "https://www.youtube.com/channel/abc",
                "https://www.youtube.com/feeds/videos.xml?channel_id=abc",
            ),
            (
                "https://www.youtube.com/user/abc",
                "https://www.youtube.com/feeds/videos.xml?user=abc",
            ),
            (
                "https://www.youtube.com/playlist?list=abc",
                "https://www.youtube.com/feeds/videos.xml?playlist_id=abc",
            ),
            (
                "https://www.reddit.com/r/abc",
                "https://www.reddit.com/r/abc.rss",
            ),
            ("https://vimeo.com/abc", "https://vimeo.com/abc/videos/rss"),
            (
                "https://github.com/orgs/community/discussions",
                "https://github.com/orgs/community/discussions.atom",
            ),
            (
                "https://github.com/orgs/other-org/discussions/31",
                "https://github.com/orgs/other-org/discussions.atom",
            ),
            (
                "https://github.com/orgs/another-org/discussions/31#discussioncomment-4048867",
                "https://github.com/orgs/another-org/discussions.atom",
            ),
            (
                "https://github.com/feedbin/feedbin/discussions",
                "https://github.com/feedbin/feedbin/discussions.atom",
            ),
            (
                "https://github.com/rails/rails/discussions/42",
                "https://github.com/rails/rails/discussions.atom",
            ),
        ];
        for (page, feed) in cases {
            let stub = Stub::default().with(page, HTML).with(feed, ATOM);
            let response = stub.download(page).await;
            assert_eq!(
                urls(KnownPattern::find(&stub, &response).await),
                [feed],
                "{page}"
            );
        }
    }

    #[tokio::test]
    async fn finds_channel_id() {
        let stub = Stub::default()
            .with(
                "https://www.youtube.com",
                r#"<head><meta itemprop="channelId" content="abc"></head>"#,
            )
            .with(
                "https://www.youtube.com/feeds/videos.xml?channel_id=abc",
                ATOM,
            );
        let response = stub.download("https://www.youtube.com").await;
        assert_eq!(
            urls(KnownPattern::find(&stub, &response).await),
            ["https://www.youtube.com/feeds/videos.xml?channel_id=abc"]
        );
    }

    #[tokio::test]
    async fn finds_mastodon_profiles() {
        let mut stub = Stub::default().with("https://example.social/@someone.rss", ATOM);
        stub.responses.insert(
            "https://example.social/@someone".into(),
            Response {
                url: "https://example.social/@someone".into(),
                status: 200,
                headers: vec![("server".into(), "Mastodon".into())],
                body: HTML.into(),
                redirects: Vec::new(),
            },
        );
        let response = stub.download("https://example.social/@someone").await;
        assert_eq!(
            urls(KnownPattern::find(&stub, &response).await),
            ["https://example.social/@someone.rss"]
        );
    }
}

mod guess {
    use super::*;

    #[tokio::test]
    async fn guesses_feed() {
        let stub = Stub::default()
            .with("https://example.com", "")
            .with("https://example.com/feed", ATOM);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(Guess::find(&stub, &response).await),
            ["https://example.com/feed"]
        );
    }

    #[tokio::test]
    async fn guesses_rss() {
        let stub = Stub::default()
            .with("https://example.com", "")
            .with("https://example.com/rss", ATOM);
        let response = stub.download("https://example.com").await;
        assert_eq!(
            urls(Guess::find(&stub, &response).await),
            ["https://example.com/rss"]
        );
    }
}

mod feed_finder {
    use super::*;

    #[tokio::test]
    async fn finds_a_feed_url() {
        let stub = Stub::default().with("https://example.com/atom.xml", ATOM);
        assert_eq!(
            found(&stub, "https://example.com/atom.xml").await,
            ["https://example.com/atom.xml"]
        );
    }

    #[tokio::test]
    async fn adds_https_to_a_url_without_a_scheme() {
        let stub = Stub::default()
            .with("https://example.com/atom.xml", ATOM)
            .with("http://example.com/atom.xml", ATOM);
        assert_eq!(
            found(&stub, "example.com/atom.xml").await,
            ["https://example.com/atom.xml"]
        );
    }

    #[tokio::test]
    async fn falls_back_to_http_when_https_cant_connect() {
        let stub = Stub::default()
            .unreachable("https://example.com/atom.xml")
            .with("http://example.com/atom.xml", ATOM);
        assert_eq!(
            found(&stub, "example.com/atom.xml").await,
            ["http://example.com/atom.xml"]
        );
        assert_eq!(
            *stub.requested.lock().unwrap(),
            [
                "https://example.com/atom.xml",
                "http://example.com/atom.xml"
            ]
        );

        let stub = Stub::default().with("http://example.com/atom.xml", ATOM);
        let result = find(&stub, "example.com/atom.xml").await;
        assert!(
            matches!(result, Err(FindError::Status { status: 404, .. })),
            "an https error page isn't a reason to try http"
        );
    }

    #[tokio::test]
    async fn stops_at_the_first_source_that_finds_feeds() {
        let stub = Stub::default()
            .with(
                "https://example.com",
                r#"<link rel="alternate" type="application/atom+xml" href="/atom.xml">
                   <a href="/rss">RSS</a>"#,
            )
            .with("https://example.com/atom.xml", ATOM)
            .with("https://example.com/rss", ATOM);
        assert_eq!(
            found(&stub, "https://example.com").await,
            ["https://example.com/atom.xml"]
        );
        let requested = stub.requested.lock().unwrap();
        assert!(!requested.contains(&"https://example.com/rss".to_string()));
    }

    #[tokio::test]
    async fn falls_through_to_later_sources() {
        let stub = Stub::default()
            .with("https://example.com", r#"<a href="/about">About</a>"#)
            .with("https://example.com/feed", ATOM);
        assert_eq!(
            found(&stub, "https://example.com").await,
            ["https://example.com/feed"]
        );
    }

    #[tokio::test]
    async fn finds_nothing_on_a_page_without_feeds() {
        let stub = Stub::default().with("https://example.com", HTML);
        assert!(found(&stub, "https://example.com").await.is_empty());
    }

    #[tokio::test]
    async fn fails_when_the_url_itself_fails() {
        let stub = Stub::default();
        let result = find(&stub, "https://example.com").await;
        assert!(matches!(result, Err(FindError::Status { status: 404, .. })));
    }

    #[test]
    fn lists_options_without_fetching() {
        let response = Response {
            url: "https://example.com/".into(),
            status: 200,
            headers: Vec::new(),
            redirects: Vec::new(),
            body: br#"<link rel="alternate" type="application/rss+xml" href="/rss.xml">
                      <a href="/atom">Atom</a>"#
                .to_vec(),
        };
        assert_eq!(find_options(&response), ["https://example.com/rss.xml"]);

        let response = Response {
            body: br#"<a href="/atom">Atom</a><a href="/about">About</a>"#.to_vec(),
            ..response
        };
        assert_eq!(find_options(&response), ["https://example.com/atom"]);
    }

    #[test]
    fn finding_can_move_between_threads() {
        fn assert_send<T: Send>(_: T) {}
        let fetcher = SendStub;
        assert_send(find(&fetcher, "https://example.com"));
    }

    /// A `Sync` fetcher, for checking that `find`'s future is `Send`.
    struct SendStub;

    impl Fetch for SendStub {
        async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
            Err(FetchError::Request(request.url.clone()))
        }
    }
}

async fn found(stub: &Stub, url: &str) -> Vec<String> {
    urls(find(stub, url).await.unwrap())
}

fn urls(found: Vec<mariner_feeds::finder::FoundFeed>) -> Vec<String> {
    found.into_iter().map(|found| found.url).collect()
}

/// Serves registered bodies with status 200, can't connect to unreachable
/// URLs, and serves 404 for anything else.
#[derive(Default)]
struct Stub {
    responses: HashMap<String, Response>,
    /// URLs that can't connect.
    unreachable: Vec<String>,
    requested: Mutex<Vec<String>>,
}

impl Stub {
    fn unreachable(mut self, url: &str) -> Self {
        self.unreachable.push(normalize(url));
        self
    }

    fn with(mut self, url: &str, body: &str) -> Self {
        let url = normalize(url);
        self.responses.insert(
            url.clone(),
            Response {
                url,
                status: 200,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
                redirects: Vec::new(),
            },
        );
        self
    }

    /// Fetches `url` the way a source receives it.
    async fn download(&self, url: &str) -> Response {
        self.fetch(&Request::get(url)).await.unwrap()
    }
}

impl Fetch for Stub {
    async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
        let url = normalize(&request.url);
        self.requested.lock().unwrap().push(url.clone());
        if self.unreachable.contains(&url) {
            return Err(FetchError::Connect("no route".into()));
        }
        Ok(self.responses.get(&url).cloned().unwrap_or(Response {
            url,
            status: 404,
            ..Response::default()
        }))
    }
}

fn normalize(url: &str) -> String {
    url::Url::parse(url).unwrap().to_string()
}
