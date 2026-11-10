//! uproxx.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("uproxx.com")
        .title(Field::new().selector("div.entry-header h1"))
        .author(Field::new().attr(r#"meta[name="qc:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".entry-content")
                .rename("div.image", "figure")
                .rename("div.image .wp-media-credit", "figcaption"),
        )
}
