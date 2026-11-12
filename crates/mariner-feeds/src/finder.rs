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

use crate::Feed;
use crate::fetch::{Fetch, FetchError, Request, Response};

pub mod source;

pub use source::{BodyLinks, Guess, KnownPattern, MetaLinks, Source, Xml};

/// A feed found for a URL.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundFeed {
    /// The URL the feed was fetched from, after redirects.
    pub url: String,
    pub feed: Feed,
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

/// Finds the feeds for `url`, which may lack a scheme (`example.com`): then
/// it's tried over `https`, and over `http` only if that can't connect.
/// Fails only when `url` itself can't be fetched; finding nothing is an
/// empty list.
pub async fn find(fetcher: &impl Fetch, url: &str) -> Result<Vec<FoundFeed>, FindError> {
    let candidates = clean_urls(url).ok_or_else(|| FindError::InvalidUrl(url.to_string()))?;
    let mut failure = None;
    for url in candidates {
        match fetcher.fetch(&Request::get(&url)).await {
            Ok(response) => return find_in(fetcher, url, response).await,
            Err(source) => failure = Some(FindError::Fetch { url, source }),
        }
    }
    Err(failure.expect("there's always a URL to try"))
}

/// The feeds on the page `url` returned.
async fn find_in(
    fetcher: &impl Fetch,
    url: String,
    response: Response,
) -> Result<Vec<FoundFeed>, FindError> {
    if !response.is_success() {
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

/// The URLs to try for what was typed: with no scheme, `https://` and then
/// `http://`; with a scheme other than HTTP(S) (`feed://`), `http`;
/// otherwise the URL itself.
fn clean_urls(url: &str) -> Option<Vec<String>> {
    let url = url.trim();
    if !url.contains("://") {
        let rest = url.trim_start_matches('/');
        return ["https", "http"]
            .into_iter()
            .map(|scheme| parse_web_url(&format!("{scheme}://{rest}")))
            .collect();
    }
    let mut parsed = Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        let rest = &url[url.find("://")? + 3..];
        parsed = Url::parse(&format!("http://{rest}")).ok()?;
    }
    parsed.host_str()?;
    Some(vec![parsed.to_string()])
}

/// `url`, normalized, when it's a web URL with a host.
fn parse_web_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    parsed.host_str()?;
    Some(parsed.to_string())
}
