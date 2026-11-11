//! Fetching over the network, behind a trait, so callers choose how: with
//! [`HttpFetcher`] (reqwest), or with their platform's own HTTP client.

use crate::{Feed, parse_with_charset};

#[cfg(feature = "fetch")]
mod http;
#[cfg(feature = "fetch")]
pub use http::HttpFetcher;

/// Fetches requests, following redirects and recording each one. Futures
/// must be `Send`, so callers can await them on any thread.
pub trait Fetch {
    fn fetch(&self, request: &Request)
    -> impl Future<Output = Result<Response, FetchError>> + Send;
}

/// A GET request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Request {
            url: url.into(),
            headers: Vec::new(),
        }
    }

    /// Adds a header when `value` is present.
    pub fn header(mut self, name: &str, value: Option<&str>) -> Self {
        if let Some(value) = value {
            self.headers.push((name.to_string(), value.to_string()));
        }
        self
    }
}

/// A fetched response.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The final URL, after redirects.
    pub url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Each redirect followed on the way, in order.
    pub redirects: Vec<Redirect>,
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

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The body as a feed, when it is one.
    pub fn feed(&self) -> Option<Feed> {
        parse_with_charset(&self.body, self.charset()).ok()
    }
}

/// One redirect followed while fetching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub status: u16,
    pub from: String,
    pub to: String,
}

impl Redirect {
    /// 301 and 308 say the resource has moved for good.
    pub fn is_permanent(&self) -> bool {
        matches!(self.status, 301 | 308)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("timed out")]
    Timeout,
    #[error("couldn't connect: {0}")]
    Connect(String),
    #[error("{0}")]
    Request(String),
}
