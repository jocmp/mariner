//! The app's HTTP client, as core's [`Fetch`].

use std::sync::Arc;

use mariner_feeds::fetch::{self, Fetch, FetchError, Request, Response};

/// The app's HTTP client. It sends one GET and returns the response as it
/// came, without following redirects: core follows them itself, so it can
/// record each one (a feed that moved for good gets its new URL).
///
/// Bodies should be decompressed, as URLSession and OkHttp do on their
/// own. Set a timeout; a request that never ends holds up a refresh.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, HttpError>;
}

/// A GET request.
#[derive(Debug, Clone, uniffi::Record)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<HttpHeader>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

/// Why a request got no response.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HttpError {
    #[error("timed out")]
    Timeout,
    #[error("couldn't connect: {message}")]
    Connect { message: String },
    #[error("{message}")]
    Request { message: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for HttpError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        HttpError::Request {
            message: error.reason,
        }
    }
}

/// Core's [`Fetch`] over the app's [`HttpClient`].
#[derive(Clone)]
pub(crate) struct PlatformFetcher {
    client: Arc<dyn HttpClient>,
}

impl PlatformFetcher {
    pub fn new(client: Arc<dyn HttpClient>) -> Self {
        PlatformFetcher { client }
    }

    async fn send(&self, request: Request) -> Result<Response, FetchError> {
        let url = request.url.clone();
        let response = self
            .client
            .send(request.into())
            .await
            .map_err(fetch_error)?;
        Ok(Response {
            url,
            status: response.status,
            headers: response
                .headers
                .into_iter()
                .map(|header| (header.name, header.value))
                .collect(),
            body: response.body,
            redirects: Vec::new(),
        })
    }
}

impl Fetch for PlatformFetcher {
    async fn fetch(&self, request: &Request) -> Result<Response, FetchError> {
        fetch::follow_redirects(request, |hop| self.send(hop)).await
    }
}

impl From<Request> for HttpRequest {
    fn from(request: Request) -> Self {
        HttpRequest {
            url: request.url,
            headers: request
                .headers
                .into_iter()
                .map(|(name, value)| HttpHeader { name, value })
                .collect(),
        }
    }
}

fn fetch_error(error: HttpError) -> FetchError {
    match error {
        HttpError::Timeout => FetchError::Timeout,
        HttpError::Connect { message } => FetchError::Connect(message),
        HttpError::Request { message } => FetchError::Request(message),
    }
}
