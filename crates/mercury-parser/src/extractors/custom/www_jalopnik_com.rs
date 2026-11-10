//! www.jalopnik.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.jalopnik.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="article:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("article.news-post")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform(".slide-key", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([".breadcrumbs", ".byline-container"]),
        )
}
