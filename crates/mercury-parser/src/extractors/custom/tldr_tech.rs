//! tldr.tech

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("tldr.tech")
        .title(Field::new().selector("h1"))
        .lead_image_url(Field::new().attr(r#"meta[name="twitter:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".content-center", "body"])
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                }),
        )
}
