//! actualidad.rt.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("actualidad.rt.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="article:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="mediator_published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".ArticleView-text")
                .clean([".ReadMore-root", "source"]),
        )
}
