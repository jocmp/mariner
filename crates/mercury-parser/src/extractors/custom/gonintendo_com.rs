//! gonintendo.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("gonintendo.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="og:article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".content")
                .clean([".text-brand-gray-600"]),
        )
}
