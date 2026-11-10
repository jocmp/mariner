//! www.versants.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.versants.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .transform(".featured-image", |node| {
                    node.add_class("mercury-parser-keep");
                    let figcaption = node.select("span");
                    let figures = node.select("figure");
                    if figures.exists() {
                        figures.append_selection(&figcaption);
                    }
                    None
                })
                .selector(".article-content")
                .clean([
                    ".adv-link",
                    ".versa-target",
                    "header",
                    ".author",
                    ".thumbnail-slider",
                ]),
        )
}
