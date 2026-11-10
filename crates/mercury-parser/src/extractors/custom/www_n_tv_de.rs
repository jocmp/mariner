//! www.n-tv.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.n-tv.de")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="date"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".article__text", "article"])
                .clean([".article__share-main"]),
        )
}
