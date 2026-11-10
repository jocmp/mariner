//! chicagoyimby.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("chicagoyimby.com")
        .title(Field::new().selector("h1.post-title"))
        .author(Field::new().selector(".entry-meta-author a"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".entry-content")
                .transform("img", |node| {
                    node.remove_attr("sizes");
                    None
                })
                .clean([".breadcrumb"]),
        )
}
