//! `src/extractors/collect-all-pages.js`: follow next-page links and
//! append each page's content.

use url::Url;

use super::CustomExtractor;
use super::generic;
use super::root_extractor::{self, Context, Extracted};
use crate::error::Error;
use crate::parser::ParseResult;
use crate::resource;
use crate::utils::dom::inner_html;
use crate::utils::text::remove_anchor;

// If we've gone over 26 pages, something has likely gone wrong.
const MAX_PAGES: usize = 26;

/// Fetch every page after the first, starting at `next_page_url`, and
/// append its content to `result`.
pub async fn collect_all_pages(
    extractor: Option<&CustomExtractor>,
    mut result: ParseResult,
    next_page_url: String,
    url: &str,
    meta_cache: &[String],
) -> Result<ParseResult, Error> {
    // At this point, we've fetched just the first page.
    let mut pages = 1;
    let mut previous_urls = vec![remove_anchor(url)];
    let mut next_page_url = Some(next_page_url);

    while let Some(next_url) = next_page_url.take() {
        if pages >= MAX_PAGES {
            break;
        }
        pages += 1;

        let Ok(parsed_url) = Url::parse(&next_url) else {
            break;
        };
        let response = resource::fetch_resource(&next_url, &[]).await?;
        let page = extract_page(
            extractor,
            &next_url,
            &parsed_url,
            &response,
            meta_cache,
            &previous_urls,
        )?;
        previous_urls.push(next_url);

        // Upstream interpolates missing content as "null".
        result.content = Some(format!(
            "{}<hr><h4>Page {pages}</h4>{}",
            result.content.as_deref().unwrap_or("null"),
            page.content.as_deref().unwrap_or("null")
        ));
        next_page_url = page.next_page_url;
    }

    result.total_pages = pages;
    result.rendered_pages = pages;
    result.word_count = Some(generic::word_count::extract(&format!(
        "<div>{}</div>",
        result.content.as_deref().unwrap_or_default()
    )));

    Ok(result)
}

// Kept synchronous so the (non-`Send`) document never lives across an
// await.
fn extract_page(
    extractor: Option<&CustomExtractor>,
    url: &str,
    parsed_url: &Url,
    response: &resource::Response,
    meta_cache: &[String],
    previous_urls: &[String],
) -> Result<Extracted, Error> {
    let doc = resource::from_bytes(&response.body, &response.content_type)?;
    let html = inner_html(&doc.root());

    let ctx = Context {
        doc: &doc,
        url,
        parsed_url,
        html: &html,
        meta_cache,
        fallback: true,
        previous_urls,
    };
    Ok(root_extractor::extract(extractor, &ctx))
}
