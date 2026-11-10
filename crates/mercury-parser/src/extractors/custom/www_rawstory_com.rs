//! www.rawstory.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.rawstory.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector(".blog-title"),
        )
        .author(Field::new().selectors([
            "div.main-post-head .social-author__name",
            ".blog-author a:first-of-type",
        ]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selector(".blog-author a:last-of-type")
                .timezone("EST"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([".post-body", ".blog-content"]))
}
