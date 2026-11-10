//! www.westernjournalism.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.westernjournalism.com")
        .title(Field::new().selectors(["title", "h1.entry-title"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="DC.date.issued"]"#, "value"))
        .dek(Field::new().selector(".subtitle"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.article-sharing.top + div")
                .clean([".ad-notice-small"]),
        )
}
