//! www.prospectmagazine.co.uk

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.prospectmagazine.co.uk")
        .title(Field::new().selectors([".blog-header__title", ".page-title"]))
        .author(Field::new().selectors([".blog-header__author-link", ".aside_author .title"]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selector(".post-info")
                .timezone("Europe/London"),
        )
        .dek(Field::new().selectors([".blog-header__description", ".page-subtitle"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([".blog__container", "article .post_content"]))
}
