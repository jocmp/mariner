//! Finds the feeds for a URL the user typed: the URL itself if it's a
//! feed, otherwise the feeds its page points to.
//!
//! Sources are tried in order, and the first one that finds anything wins:
//!
//! 1. The response itself, when it parses as a feed.
//! 2. `<link rel="alternate">` tags with a feed type.
//! 3. Known URL patterns (YouTube, Reddit, Vimeo, GitHub discussions), a
//!    YouTube page's channel id, or a Mastodon profile.
//! 4. The first four `<a>` links whose URL mentions `feed`, `xml`, `rss`
//!    or `atom`.
//! 5. Guesses: `/rss` and `/feed` on the same host.
//!
//! Candidates that fail to fetch or don't parse as feeds are skipped.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

use crate::{Feed, parse_with_charset};

#[cfg(feature = "fetch")]
mod http;
#[cfg(feature = "fetch")]
pub use http::HttpFetcher;

/// A feed found for a URL.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundFeed {
    /// The URL the feed was fetched from, after redirects.
    pub url: String,
    pub feed: Feed,
}

/// A fetched response.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The final URL, after redirects.
    pub url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The `charset` parameter of the `Content-Type` header.
    fn charset(&self) -> Option<&str> {
        let content_type = self.header("content-type")?;
        content_type.split(';').skip(1).find_map(|parameter| {
            let (key, value) = parameter.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("charset")
                .then(|| value.trim().trim_matches('"'))
        })
    }

    fn feed(&self) -> Option<Feed> {
        parse_with_charset(&self.body, self.charset()).ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("{0}")]
    Request(String),
}

/// Fetches URLs for the finder, following redirects.
pub trait Fetch {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Response, FetchError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum FindError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("couldn't fetch {url}: {source}")]
    Fetch { url: String, source: FetchError },
    #[error("{url} returned HTTP {status}")]
    Status { url: String, status: u16 },
}

/// Finds the feeds for `url`, which may lack a scheme (`example.com`).
/// Fails only when `url` itself can't be fetched; finding nothing is an
/// empty list.
pub async fn find(fetcher: &impl Fetch, url: &str) -> Result<Vec<FoundFeed>, FindError> {
    let url = clean_url(url).ok_or_else(|| FindError::InvalidUrl(url.to_string()))?;
    let response = fetcher
        .fetch(&url)
        .await
        .map_err(|source| FindError::Fetch {
            url: url.clone(),
            source,
        })?;
    if !(200..300).contains(&response.status) {
        return Err(FindError::Status {
            url,
            status: response.status,
        });
    }

    if let Some(feed) = response.feed() {
        return Ok(vec![FoundFeed {
            url: response.url.clone(),
            feed,
        }]);
    }
    let page = Page::new(&response);
    let finders: [fn(&Page) -> Vec<String>; 4] = [meta_links, known_patterns, body_links, guesses];
    for candidates in finders {
        let found = fetch_feeds(fetcher, candidates(&page)).await;
        if !found.is_empty() {
            return Ok(found);
        }
    }
    Ok(Vec::new())
}

/// Feed URLs a page links to, without fetching them: its
/// `<link rel="alternate">` feeds, or else its feed-like `<a>` links.
pub fn find_options(response: &Response) -> Vec<String> {
    let page = Page::new(response);
    let options = meta_links(&page);
    if options.is_empty() {
        body_links(&page)
    } else {
        options
    }
}

/// Adds `http://` when the URL has no scheme, and replaces a scheme
/// other than HTTP(S) with `http`.
fn clean_url(url: &str) -> Option<String> {
    let url = url.trim();
    let with_scheme = if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{}", url.trim_start_matches('/'))
    };
    let mut parsed = Url::parse(&with_scheme).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        let rest = &with_scheme[with_scheme.find("://")? + 3..];
        parsed = Url::parse(&format!("http://{rest}")).ok()?;
    }
    parsed.host_str()?;
    Some(parsed.to_string())
}

/// Fetches each candidate once, keeping those that parse as feeds.
async fn fetch_feeds(fetcher: &impl Fetch, candidates: Vec<String>) -> Vec<FoundFeed> {
    let mut found: Vec<FoundFeed> = Vec::new();
    let mut tried: Vec<String> = Vec::new();
    for candidate in candidates {
        if tried.contains(&candidate) {
            continue;
        }
        tried.push(candidate.clone());
        let Ok(response) = fetcher.fetch(&candidate).await else {
            continue;
        };
        if !(200..300).contains(&response.status) {
            continue;
        }
        if let Some(feed) = response.feed()
            && !found.iter().any(|existing| existing.url == response.url)
        {
            found.push(FoundFeed {
                url: response.url,
                feed,
            });
        }
    }
    found
}

/// The fetched page as HTML.
struct Page<'a> {
    response: &'a Response,
    base: Option<Url>,
    document: dom_query::Document,
}

impl<'a> Page<'a> {
    fn new(response: &'a Response) -> Self {
        Page {
            response,
            base: Url::parse(&response.url).ok(),
            document: dom_query::Document::from(String::from_utf8_lossy(&response.body).as_ref()),
        }
    }

    fn join(&self, href: &str) -> Option<String> {
        let base = self.base.as_ref()?;
        base.join(href.trim()).ok().map(String::from)
    }
}

fn meta_links(page: &Page) -> Vec<String> {
    const TYPES: [&str; 4] = [
        "application/rss+xml",
        "application/atom+xml",
        "application/feed+json",
        "application/json",
    ];
    page.document
        .select("link[rel~=alternate]")
        .iter()
        .filter_map(|link| {
            let kind = link.attr("type")?.trim().to_ascii_lowercase();
            let href = link.attr("href")?;
            TYPES
                .contains(&kind.as_str())
                .then(|| page.join(&href))
                .flatten()
        })
        .collect()
}

fn known_patterns(page: &Page) -> Vec<String> {
    static PATTERNS: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
        [
            (
                r"https://www\.youtube\.com/channel/([^/#?]*)",
                "https://www.youtube.com/feeds/videos.xml?channel_id=%s",
            ),
            (
                r"https://www\.youtube\.com/user/([^/#?]*)",
                "https://www.youtube.com/feeds/videos.xml?user=%s",
            ),
            (
                r"https://www\.youtube\.com/playlist\?list=([^&]*)",
                "https://www.youtube.com/feeds/videos.xml?playlist_id=%s",
            ),
            (
                r"https?://(?:www\.)?reddit\.com/r/([^/#?]*)",
                "https://www.reddit.com/r/%s.rss",
            ),
            (
                r"https://vimeo\.com/([^/#?]*)",
                "https://vimeo.com/%s/videos/rss",
            ),
            (
                r"https://github\.com/(orgs/[^/#?]+/discussions)",
                "https://github.com/%s.atom",
            ),
            (
                r"https://github\.com/([^/#?]+/[^/#?]+/discussions)",
                "https://github.com/%s.atom",
            ),
        ]
        .into_iter()
        .map(|(pattern, template)| (Regex::new(pattern).unwrap(), template))
        .collect()
    });
    static CHANNEL_ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""channelId":"(.*?)""#).unwrap());

    let url = &page.response.url;
    if let Some(feed) = PATTERNS.iter().find_map(|(pattern, template)| {
        let id = pattern.captures(url)?.get(1)?.as_str().to_string();
        (!id.is_empty()).then(|| template.replace("%s", &id))
    }) {
        return vec![feed];
    }
    if url.starts_with("https://www.youtube.com") {
        let id = page
            .document
            .select("meta[itemprop='channelId']")
            .attr("content")
            .map(|id| id.to_string())
            .or_else(|| {
                let body = String::from_utf8_lossy(&page.response.body);
                CHANNEL_ID.captures(&body).map(|c| c[1].to_string())
            });
        if let Some(id) = id {
            return vec![format!(
                "https://www.youtube.com/feeds/videos.xml?channel_id={id}"
            )];
        }
    }
    let mastodon = page
        .response
        .headers
        .iter()
        .any(|(_, value)| value.to_ascii_lowercase().contains("mastodon"));
    if mastodon {
        return vec![format!("{url}.rss")];
    }
    Vec::new()
}

fn body_links(page: &Page) -> Vec<String> {
    const HINTS: [&str; 4] = ["feed", "xml", "rss", "atom"];
    page.document
        .select("a[href]")
        .iter()
        .filter_map(|anchor| {
            let href = anchor.attr("href")?;
            HINTS
                .iter()
                .any(|hint| href.contains(hint))
                .then(|| page.join(&href))
                .flatten()
        })
        .take(4)
        .collect()
}

fn guesses(page: &Page) -> Vec<String> {
    let Some(base) = &page.base else {
        return Vec::new();
    };
    let Some(host) = base.host_str() else {
        return Vec::new();
    };
    ["/rss", "/feed"]
        .iter()
        .map(|path| format!("{}://{host}{path}", base.scheme()))
        .collect()
}
