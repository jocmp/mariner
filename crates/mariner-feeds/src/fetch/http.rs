//! A [`Fetch`] over HTTP.

use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue, LOCATION};
use reqwest::redirect::Policy;

use super::{Fetch, FetchError, Redirect, Request, Response};

const TIMEOUT: Duration = Duration::from_secs(30);
/// Names the app and where to learn about it, as some hosts (Wikimedia,
/// for one) require.
const USER_AGENT: &str = concat!(
    "Mariner/",
    env!("CARGO_PKG_VERSION"),
    " (+https://jocmp.com)"
);
/// Redirects followed before giving up.
const MAX_REDIRECTS: usize = 10;

/// Fetches with reqwest, decompressing bodies. It follows redirects itself,
/// so it can record each one.
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
        let mut url =
            reqwest::Url::parse(&request.url).map_err(|e| FetchError::Request(e.to_string()))?;
        let mut redirects = Vec::new();
        loop {
            let mut builder = self.client.get(url.clone());
            for (name, value) in &request.headers {
                if let (Ok(name), Ok(value)) = (
                    HeaderName::try_from(name.as_str()),
                    HeaderValue::try_from(value.as_str()),
                ) {
                    builder = builder.header(name, value);
                }
            }
            let response = builder.send().await.map_err(error)?;
            let status = response.status();
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|location| url.join(location).ok());
            if status.is_redirection()
                && let Some(next) = location
            {
                if redirects.len() == MAX_REDIRECTS {
                    return Err(FetchError::Request(format!(
                        "more than {MAX_REDIRECTS} redirects"
                    )));
                }
                redirects.push(Redirect {
                    status: status.as_u16(),
                    from: url.to_string(),
                    to: next.to_string(),
                });
                url = next;
                continue;
            }
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.to_string(), value.to_str().ok()?.to_string()))
                })
                .collect();
            let body = response.bytes().await.map_err(error)?.to_vec();
            return Ok(Response {
                url: url.to_string(),
                status: status.as_u16(),
                headers,
                body,
                redirects,
            });
        }
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
