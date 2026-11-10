//! Errors returned by [`crate::Parser`].

/// Why a page couldn't be parsed. Messages match upstream's.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "The url parameter passed does not look like a valid URL. Please check your URL and try again."
    )]
    InvalidUrl,
    /// The request failed, or the response was rejected (non-200 status,
    /// disallowed content type, too large).
    #[error("{0}")]
    Fetch(String),
    #[error("Content does not appear to be text.")]
    NotText,
    #[error("No children, likely a bad parse.")]
    BadParse,
    /// No HTML was given and the crate was built without the `fetch`
    /// feature.
    #[error("Fetching is disabled; pass the page HTML in ParseOptions::html.")]
    FetchDisabled,
}
