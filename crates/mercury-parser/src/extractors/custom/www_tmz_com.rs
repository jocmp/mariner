//! www.tmz.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.tmz.com")
        .title(Field::new().selectors([".post-title-breadcrumb", "h1", ".headline"]))
        .author(Field::literal("TMZ STAFF"))
        .date_published(
            Field::new()
                .selectors([".article__published-at", ".article-posted-date"])
                .timezone("America/Los_Angeles"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".article__blocks", ".article-content", ".all-post-body"])
                .clean([".lightbox-link"]),
        )
}
