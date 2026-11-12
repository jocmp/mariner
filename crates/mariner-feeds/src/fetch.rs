//! Fetching over the network, behind a trait, so callers choose how: with
//! [`HttpFetcher`] (reqwest), or with their platform's own HTTP client.

use url::Url;

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

/// Redirects followed before giving up.
const MAX_REDIRECTS: usize = 10;

/// Fetches `request` with `send`, which makes one request without following
/// redirects, following each redirect itself and recording it. This is
/// [`Fetch`] for HTTP clients that make one request at a time.
pub async fn follow_redirects<S, F>(request: &Request, mut send: S) -> Result<Response, FetchError>
where
    S: FnMut(Request) -> F,
    F: Future<Output = Result<Response, FetchError>>,
{
    let mut url = Url::parse(&request.url).map_err(|e| FetchError::Request(e.to_string()))?;
    let mut redirects = Vec::new();
    loop {
        let hop = Request {
            url: url.to_string(),
            headers: request.headers.clone(),
        };
        let mut response = send(hop).await?;
        let next = response
            .header("location")
            .and_then(|location| url.join(location).ok());
        match next {
            Some(next) if (300..400).contains(&response.status) => {
                if redirects.len() == MAX_REDIRECTS {
                    return Err(FetchError::Request(format!(
                        "more than {MAX_REDIRECTS} redirects"
                    )));
                }
                redirects.push(Redirect {
                    status: response.status,
                    from: url.to_string(),
                    to: next.to_string(),
                });
                url = next;
            }
            _ => {
                response.url = url.to_string();
                response.redirects = redirects;
                return Ok(response);
            }
        }
    }
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
