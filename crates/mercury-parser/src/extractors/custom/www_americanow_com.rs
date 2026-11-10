//! www.americanow.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.americanow.com")
        .title(
            Field::new()
                .selector(".title")
                .attr(r#"meta[name="title"]"#, "value"),
        )
        .author(Field::new().selector(".byline"))
        .date_published(Field::new().attr(r#"meta[name="publish_date"]"#, "value"))
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".article-content", ".image", ".body"])
                .selector(".body")
                .clean([".article-video-wrapper", ".show-for-small-only"]),
        )
}
