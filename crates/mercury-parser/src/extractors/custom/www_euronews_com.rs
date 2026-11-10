//! www.euronews.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.euronews.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector(".c-article-contributors"),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector("h2.c-article-summary"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".c-article-content", "article"])
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform(".widget__figure", |node| {
                    node.add_class("mercury-parser-keep");
                    None
                })
                .clean([".c-ad", ".c-widget-related", ".connatix-container"]),
        )
}
