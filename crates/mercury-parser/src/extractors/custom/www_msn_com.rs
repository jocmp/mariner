//! www.msn.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.msn.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector("span.authorname-txt"))
        .content(
            Field::new()
                .selector("div.richtext")
                .clean(["span.caption"]),
        )
        .date_published(Field::new().selector("span.time"))
        .lead_image_url(Field::new())
        .dek(Field::new())
}
