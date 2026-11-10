//! www.futura-sciences.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.futura-sciences.com")
        .title(Field::new().selectors(["title", "h1"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["#article-anchor-article-main-content", ".article-text"])
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
                .transform("ul", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([".cWHWfD", r#"span[class*="wrappers__Span"]"#]),
        )
}
