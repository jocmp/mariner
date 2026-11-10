//! The generic extractor, from `src/extractors/generic/`: the default for
//! every domain without a custom extractor, and the fallback for fields a
//! custom extractor doesn't cover.

pub mod author;
pub mod content;
pub mod date_published;
pub mod direction;
pub mod excerpt;
pub mod lead_image_url;
pub mod next_page_url;
pub mod title;
pub mod url;
pub mod word_count;

use super::root_extractor::{Context, Extracted};

/// Run every generic field extractor, in upstream's order.
pub fn extract(ctx: &Context<'_>) -> Extracted {
    let title = title::extract(ctx.doc, ctx.url, ctx.meta_cache);
    let date_published = date_published::extract(ctx.doc, ctx.url, ctx.meta_cache);
    let author = author::extract(ctx.doc, ctx.meta_cache);
    let content = content::extract(ctx.doc, ctx.html, Some(&title), ctx.url);
    let lead_image_url =
        lead_image_url::extract(ctx.doc, content.as_deref(), ctx.meta_cache, ctx.html);
    let next_page_url = next_page_url::extract(ctx.doc, ctx.url, ctx.parsed_url, ctx.previous_urls);
    let excerpt = excerpt::extract(ctx.doc, content.as_deref(), ctx.meta_cache);
    let word_count = word_count::extract(content.as_deref().unwrap_or_default());
    let direction = direction::direction(&title).to_string();
    let (url, domain) = url::extract(ctx.doc, ctx.url, ctx.meta_cache);

    Extracted {
        title: Some(title),
        content,
        author,
        date_published,
        lead_image_url,
        dek: None,
        next_page_url,
        url: Some(url),
        domain,
        excerpt: Some(excerpt),
        word_count: Some(word_count),
        direction: Some(direction),
        extended: Vec::new(),
    }
}
