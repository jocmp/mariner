//! scan.netsecurity.ne.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("scan.netsecurity.ne.jp")
        .title(Field::new().selector("header.arti-header h1.head"))
        .date_published(Field::new().attr(r#"meta[name="article:modified_time"]"#, "value"))
        .dek(Field::new().selector("header.arti-header p.arti-summary"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.arti-content.arti-content--thumbnail")
                .default_cleaner(false)
                .clean(["aside.arti-giga"]),
        )
}
