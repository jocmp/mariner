//! www.ossnews.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.ossnews.jp")
        .title(Field::new().selector("#alpha-block h1.hxnewstitle"))
        .date_published(Field::new().selector("p.fs12"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#alpha-block .section:has(h1.hxnewstitle)")
                .default_cleaner(false),
        )
}
