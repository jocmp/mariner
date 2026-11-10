//! www.tweaktown.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.tweaktown.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(r#".info-bar-div2 a[rel="author"]"#))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("#article-body"))
}
