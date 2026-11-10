//! mobilesyrup.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("mobilesyrup.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".article-content").transform(
            ".article-content > ul",
            |node| {
                node.set_attr("class", "mercury-parser-keep");
                None
            },
        ))
}
