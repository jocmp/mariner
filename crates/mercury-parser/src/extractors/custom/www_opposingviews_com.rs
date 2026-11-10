//! www.opposingviews.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.opposingviews.com")
        .title(Field::new().selectors(["h1.m-detail-header--title", "h1.title"]))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selector("div.date span span a"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="published"]"#, "value")
                .attr(r#"meta[name="publish_date"]"#, "value"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".m-detail--body", ".article-content"])
                .clean([".show-for-small-only"]),
        )
}
