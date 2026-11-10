//! www.rosa.gr

use std::sync::LazyLock;

use dom_query::Selection;
use regex::Regex;

use crate::extractors::{CustomExtractor, Field};

static PLATFORM_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(X|Facebook|Instagram|TikTok|YouTube)$").unwrap());

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.rosa.gr")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["#hero .post-image", "#post .post-text"])
                .selector("#post .post-text")
                // The hero image is a direct child; related-post thumbnails are wrapped in links.
                .rename(".post-image:has(> img)", "figure")
                .transform(".post-image .image-credits", image_credits)
                .clean([".short-read-also"]),
        )
}

// Embed credits are often just the source platform's name.
fn image_credits(node: &Selection<'_>) -> Option<String> {
    if PLATFORM_LABEL.is_match(node.text().trim()) {
        node.remove();
        return None;
    }

    Some("figcaption".to_string())
}
