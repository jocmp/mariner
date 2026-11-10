//! `src/resource/utils/fetch-resource.js`.

use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use reqwest::header::{
    CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, USER_AGENT,
};

use crate::error::Error;

const REQUEST_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";

// Connect timeout; `MAX_FETCH_TIME` bounds the whole request.
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_FETCH_TIME: Duration = Duration::from_secs(30);

// The maximum size an article can be for us to attempt parsing: 5 MB.
const MAX_CONTENT_LENGTH: usize = 5_242_880;

// Content types that we do not extract content from.
static BAD_CONTENT_TYPES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)^(audio/mpeg|image/gif|image/jpeg|image/jpg)$").unwrap());

/// A fetched page.
#[derive(Debug)]
pub struct Response {
    pub body: Vec<u8>,
    pub content_type: String,
}

/// Fetch `url`, following redirects and decompressing the body, and reject
/// responses Mercury shouldn't parse.
pub async fn fetch_resource(url: &str, headers: &[(String, String)]) -> Result<Response, Error> {
    let client = reqwest::Client::builder()
        .connect_timeout(FETCH_TIMEOUT)
        .read_timeout(FETCH_TIMEOUT)
        .timeout(MAX_FETCH_TIME)
        // Accept cookies, but per request, so the jar can't grow unbounded
        // across domains in a long-running process.
        .cookie_store(true)
        .build()
        .map_err(fetch_error)?;

    let mut header_map = HeaderMap::new();
    header_map.insert(USER_AGENT, HeaderValue::from_static(REQUEST_USER_AGENT));
    for (name, value) in headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            header_map.insert(name, value);
        }
    }

    let mut response = client
        .get(url)
        .headers(header_map)
        .send()
        .await
        .map_err(fetch_error)?;

    let status = response.status();
    if status != reqwest::StatusCode::OK {
        return Err(Error::Fetch(format!(
            "Resource returned a response status code of {} and resource was instructed to reject non-200 status codes.",
            status.as_u16()
        )));
    }

    let content_type = header(&response, CONTENT_TYPE).unwrap_or_default();
    if BAD_CONTENT_TYPES_RE.is_match(&content_type) {
        return Err(Error::Fetch(format!(
            "Content-type for this resource was {content_type} and is not allowed."
        )));
    }

    let too_large = || {
        Error::Fetch(format!(
            "Content for this resource was too large. Maximum content length is {MAX_CONTENT_LENGTH}."
        ))
    };
    if header(&response, CONTENT_LENGTH)
        .and_then(|length| length.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_CONTENT_LENGTH)
    {
        return Err(too_large());
    }

    // Stop once too many decompressed bytes have arrived, so an oversized
    // body or a gzip bomb can't exhaust memory.
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(fetch_error)? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_CONTENT_LENGTH {
            return Err(too_large());
        }
    }

    Ok(Response { body, content_type })
}

fn header(response: &reqwest::Response, name: HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn fetch_error(error: reqwest::Error) -> Error {
    Error::Fetch(format!(
        "Unable to fetch content. Original exception was {error}"
    ))
}
