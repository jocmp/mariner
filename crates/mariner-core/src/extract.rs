//! An article's full text, extracted from its web page with Mercury, for
//! feeds that publish only a summary.

use mariner_feeds::fetch::{Fetch, Request};
use mercury_parser::{ParseOptions, Parser};

use crate::{Error, Result};

/// Fetches the page at `url` and extracts its article as HTML, or `None`
/// when the page has no article Mercury can find. Only the first page of a
/// multi-page article is read.
pub async fn full_content(fetcher: &impl Fetch, url: &str) -> Result<Option<String>> {
    let response = fetcher.fetch(&Request::get(url)).await?;
    if !response.is_success() {
        return Err(Error::Status {
            url: url.to_string(),
            status: response.status,
        });
    }
    let html = String::from_utf8_lossy(&response.body);
    let options = ParseOptions {
        fetch_all_pages: false,
        ..ParseOptions::default()
    };
    let result = Parser::new().parse_html(&response.url, &html, &options)?;
    Ok(result.content.filter(|content| !content.trim().is_empty()))
}
