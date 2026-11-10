//! 9to5mac.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("9to5mac.com")
        .title(Field::new().selectors(["title", "h1"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("main")
                .transform("img", |node| {
                    node.remove_attr("sizes");
                    None
                })
                .clean([".post-meta"]),
        )
}
