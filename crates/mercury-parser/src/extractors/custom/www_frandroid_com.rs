//! www.frandroid.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.frandroid.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="parsely-author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("section.article-content")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    ".index-menu-wrapper",
                    ".is-gastric-kingfisher",
                    ".newsletter-form",
                    ".share",
                    ".article-footer",
                    ".js-feed-posts",
                    ".optidigital-adslot",
                    r#"[id^="optidigital-adslot"]"#,
                ]),
        )
}
