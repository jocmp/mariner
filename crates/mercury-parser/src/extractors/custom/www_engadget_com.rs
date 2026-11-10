//! www.engadget.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.engadget.com")
        .title(Field::new().selectors(["title", "h1"]))
        .author(Field::new().selector(".caas-attr-item-author"))
        .date_published(Field::new().attr("time", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".caas-body")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("blockquote noscript", |_| {
                    // Upstream compares `node.find('iframe')` against null, which never fails, so
                    // every match is converted.
                    Some("div".to_string())
                }),
        )
}
