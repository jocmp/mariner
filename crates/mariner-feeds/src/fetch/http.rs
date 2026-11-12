//! A [`Fetch`] over HTTP.

use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;

use super::{Fetch, FetchError, Request, Response, follow_redirects};

const TIMEOUT: Duration = Duration::from_secs(30);
/// Names the app and where to learn about it, as some hosts (Wikimedia,
/// for one) require.
const USER_AGENT: &str = concat!(
    "Mariner/",
    env!("CARGO_PKG_VERSION"),
    " (+https://jocmp.com)"
);

/// Fetches with reqwest, decompressing bodies. Redirects are followed by
/// [`follow_redirects`], so each one is recorded.
#[derive(Debug, Clone)]
pub struct HttpFetcher {
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(TIMEOUT)
            .timeout(TIMEOUT)
            .redirect(Policy::none())
            .build()
            .map_err(error)?;
        Ok(HttpFetcher { client })
    }
}

impl Fetch for HttpFetcher {
    async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
        follow_redirects(request, |hop| self.send(hop)).await
    }
}

impl HttpFetcher {
    /// Makes one request, without following redirects.
    async fn send(&self, request: Request) -> Result<Response, FetchError> {
        let mut builder = self.client.get(&request.url);
        for (name, value) in &request.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                builder = builder.header(name, value);
            }
        }
        let response = builder.send().await.map_err(error)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();
        let body = response.bytes().await.map_err(error)?.to_vec();
        Ok(Response {
            url: request.url,
            status,
            headers,
            body,
            redirects: Vec::new(),
        })
    }
}

fn error(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        FetchError::Timeout
    } else if error.is_connect() {
        FetchError::Connect(error.to_string())
    } else {
        FetchError::Request(error.to_string())
    }
}
