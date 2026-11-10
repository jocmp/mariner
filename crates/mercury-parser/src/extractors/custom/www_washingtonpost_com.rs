//! www.washingtonpost.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.washingtonpost.com")
        .title(Field::new().selectors(["h1", "#topper-headline-wrapper"]))
        .author(Field::new().selector(".pb-author-name"))
        .date_published(
            Field::new().attr(r#".author-timestamp[itemprop="datePublished"]"#, "content"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".article-body")
                .transform("div.inline-content", |node| {
                    if node.select("img,iframe,video").exists() {
                        return Some("figure".to_string());
                    }

                    node.remove();
                    None
                })
                .rename(".pb-caption", "figcaption")
                .clean([".interstitial-link", ".newsletter-inline-unit"]),
        )
}
