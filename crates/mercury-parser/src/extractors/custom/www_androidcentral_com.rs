//! www.androidcentral.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.androidcentral.com")
        .title(Field::new().selectors(["h1", "h1.main-title"]))
        .author(Field::new().attr(r#"meta[name="parsely-author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#article-body")
                .clean([".intro", "blockquote"]),
        )
}
