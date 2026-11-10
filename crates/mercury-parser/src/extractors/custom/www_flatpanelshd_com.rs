//! www.flatpanelshd.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.flatpanelshd.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[itemprop="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[itemprop="datePublished"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["#zephr-anchor", "article"])
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h4", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .rename("pre", "div"),
        )
}
