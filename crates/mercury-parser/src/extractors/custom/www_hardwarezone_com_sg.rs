//! www.hardwarezone.com.sg

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.hardwarezone.com.sg")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".article-view-author-name a"))
        .date_published(
            Field::new()
                .selector(".article-view-timestamp")
                .timezone("UTC"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".content", "article"])
                .transform("img", |node| {
                    node.remove_attr("sizes");
                    None
                })
                .transform("p", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                }),
        )
}
