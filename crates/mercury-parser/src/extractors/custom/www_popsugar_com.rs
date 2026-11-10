//! www.popsugar.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.popsugar.com")
        .title(Field::new().selectors(["h2.post-title", "title-text"]))
        .author(Field::new().attr(r#"meta[name="article:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("#content").clean([
            ".share-copy-title",
            ".post-tags",
            ".reactions",
        ]))
}
