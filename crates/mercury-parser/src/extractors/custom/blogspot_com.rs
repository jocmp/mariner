//! blogspot.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("blogspot.com")
        .content(
            Field::new()
                .selector(".post-content noscript")
                .rename("noscript", "div"),
        )
        .author(Field::new().selector(".post-author-name"))
        .title(Field::new().selector(".post h2.title"))
        .date_published(Field::new().selector("span.publishdate"))
}
