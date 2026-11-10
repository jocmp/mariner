//! www.thedrive.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.thedrive.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".entry-content", "article"])
                .transform("img", |node| {
                    node.remove_attr("sizes");
                    None
                })
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    ".product-disclosure",
                    ".recurrent-newsletter-block",
                    ".pw-incontent-commerce-ad",
                    "#author-widgets",
                ]),
        )
}
