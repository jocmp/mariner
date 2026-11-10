//! www.digitalfoundry.net

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.digitalfoundry.net")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".article-text", "article"])
                .transform("iframe[data-src]", |node| {
                    if let Some(src) = node.attr("data-src") {
                        node.set_attr("src", &src);
                    }
                    None
                })
                .clean([
                    ".youtube-sub",
                    ".object-related",
                    ".poll",
                    ".insert",
                    ".see-also",
                ]),
        )
}
