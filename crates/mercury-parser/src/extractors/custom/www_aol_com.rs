//! www.aol.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.aol.com")
        .title(Field::new().selector("h1.p-article__title"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(
            Field::new()
                .selector(".p-article__byline__date")
                .timezone("America/New_York"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".article-content"))
}
