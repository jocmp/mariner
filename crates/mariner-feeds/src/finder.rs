//! Finds the feeds for a URL the user typed: the URL itself if it's a
//! feed, otherwise the feeds its page points to.
//!
//! Each way of finding feeds is a [`Source`](source::Source), tried in this
//! order; the first that finds anything wins:
//!
//! 1. [`Xml`]: the response itself, when it parses as a feed.
//! 2. [`MetaLinks`]: `<link rel="alternate">` tags with a feed type.
//! 3. [`KnownPattern`]: known URL patterns, a YouTube page's channel id,
//!    or a Mastodon profile.
//! 4. [`BodyLinks`]: feed-like `<a>` links.
//! 5. [`Guess`]: `/rss` and `/feed` on the same host.
//!
//! Candidates that fail to fetch or don't parse as feeds are skipped.

use url::Url;

use crate::{Feed, parse_with_charset};

pub mod source;

pub use source::{BodyLinks, Guess, KnownPattern, MetaLinks, Source, Xml};

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
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The `charset` parameter of the `Content-Type` header.
    pub fn charset(&self) -> Option<&str> {
        let content_type = self.header("content-type")?;
        content_type.split(';').skip(1).find_map(|parameter| {
            let (key, value) = parameter.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("charset")
                .then(|| value.trim().trim_matches('"'))
        })
    }

    /// The body as a feed, when it is one.
    pub fn feed(&self) -> Option<Feed> {
        parse_with_charset(&self.body, self.charset()).ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("{0}")]
    Request(String),
}

/// Fetches URLs for the finder, following redirects. Futures must be
/// `Send`, so callers can await them on any thread.
pub trait Fetch {
    fn fetch(&self, url: &str) -> impl Future<Output = Result<Response, FetchError>> + Send;
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

    // Each source runs only when the ones before it found nothing.
    let found = Xml::find(fetcher, &response).await;
    if !found.is_empty() {
        return Ok(found);
    }
    let found = MetaLinks::find(fetcher, &response).await;
    if !found.is_empty() {
        return Ok(found);
    }
    let found = KnownPattern::find(fetcher, &response).await;
    if !found.is_empty() {
        return Ok(found);
    }
    let found = BodyLinks::find(fetcher, &response).await;
    if !found.is_empty() {
        return Ok(found);
    }
    Ok(Guess::find(fetcher, &response).await)
}

/// Feed URLs a page links to, without fetching them: its
/// `<link rel="alternate">` feeds, or else its feed-like `<a>` links.
pub fn find_options(response: &Response) -> Vec<String> {
    let options = MetaLinks::options(response);
    if options.is_empty() {
        BodyLinks::options(response)
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
