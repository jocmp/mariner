//! www.qdaily.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.qdaily.com")
        .title(Field::new().selectors(["h2", "h2.title"]))
        .author(Field::new().selector(".name"))
        .date_published(Field::new().attr(".date.smart-date", "data-origindate"))
        .dek(Field::new().selector(".excerpt"))
        .lead_image_url(Field::new().attr(".article-detail-hd img", "src"))
        .content(
            Field::new()
                .selector(".detail")
                .clean([".lazyload", ".lazylad", ".lazylood"]),
        )
}
