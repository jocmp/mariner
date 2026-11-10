//! www.yahoo.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.yahoo.com")
        .title(Field::new().selector("header.canvas-header"))
        .author(Field::new().selector("span.provider-name"))
        .content(
            Field::new()
                .selector(".content-canvas")
                .clean([".figure-caption"]),
        )
        .date_published(Field::new().attr("time.date[datetime]", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
}
