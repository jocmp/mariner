//! www.lemonde.fr

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.lemonde.fr")
        .title(Field::new().selector("h1.article__title"))
        .author(Field::new().selector(".author__name"))
        .date_published(Field::new().attr(r#"meta[name="og:article:published_time"]"#, "value"))
        .dek(Field::new().selector(".article__desc"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".article__content")
                .clean(["figcaption"]),
        )
}
