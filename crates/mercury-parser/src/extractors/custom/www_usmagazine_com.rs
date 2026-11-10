//! www.usmagazine.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.usmagazine.com")
        .title(Field::new().selector("header h1"))
        .author(Field::new().selectors(["a.author", "a.article-byline.tracked-offpage"]))
        .date_published(
            Field::new()
                .timezone("America/New_York")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.article-content")
                .clean([".module-related"]),
        )
}
