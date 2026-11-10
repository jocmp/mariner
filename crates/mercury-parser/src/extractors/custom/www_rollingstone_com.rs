//! www.rollingstone.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.rollingstone.com")
        .title(Field::new().selectors(["h1.l-article-header__row--title", "h1.content-title"]))
        .author(Field::new().selectors(["a.c-byline__link", "a.content-author.tracked-offpage"]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selector("time.content-published-date")
                .timezone("America/New_York"),
        )
        .dek(Field::new().selectors(["h2.l-article-header__row--lead", ".content-description"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".l-article-content")
                .group([".lead-container", ".article-content"])
                .selector(".article-content")
                .clean([".c-related-links-wrapper", ".module-related"]),
        )
}
