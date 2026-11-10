//! techcrunch.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("techcrunch.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("main")
                .clean(["img.post-authors-list__author-thumb"]),
        )
}
