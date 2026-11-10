//! www.wired.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.wired.com")
        .title(Field::new().selector(r#"h1[data-testId="ContentHeaderHed"]"#))
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector(r#"a[rel="author"]"#),
        )
        .content(
            Field::new()
                .selectors(["article.article.main-content", "article.content"])
                .clean([".visually-hidden", "figcaption img.photo", ".alert-message"]),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
}
