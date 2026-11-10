//! www.channelnewsasia.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.channelnewsasia.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .selector(".link--author-profile")
                .attr(r#"meta[name="cXenseParse:author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .selector(".article-publish:not(span)")
                .format("D MMM YYYY hh:mmA")
                .timezone("Asia/Singapore"),
        )
        .dek(Field::new().selector(".content-detail__description"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(r#"section[data-title="Content"]"#)
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                }),
        )
}
