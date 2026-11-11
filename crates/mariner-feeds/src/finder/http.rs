//! A [`Fetch`] over HTTP.

use std::time::Duration;

use super::{Fetch, FetchError, Response};

const TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!("Mariner/", env!("CARGO_PKG_VERSION"));

/// Fetches with reqwest, following redirects and decompressing bodies.
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
            .build()
            .map_err(|error| FetchError::Request(error.to_string()))?;
        Ok(HttpFetcher { client })
    }
}

impl Fetch for HttpFetcher {
    async fn fetch(&self, url: &str) -> Result<Response, FetchError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| FetchError::Request(error.to_string()))?;
        let url = response.url().to_string();
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|error| FetchError::Request(error.to_string()))?
            .to_vec();
        Ok(Response {
            url,
            status,
            headers,
            body,
        })
    }
}
