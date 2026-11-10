//! pagesix.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("pagesix.com")
        .supported_domains(["nypost.com"])
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".byline"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["#featured-image-wrapper", ".entry-content"])
                .selector(".entry-content")
                .rename("#featured-image-wrapper", "figure")
                .rename(".wp-caption-text", "figcaption")
                .clean([".modal-trigger"]),
        )
}
