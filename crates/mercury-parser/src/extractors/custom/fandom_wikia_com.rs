//! fandom.wikia.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("fandom.wikia.com")
        .title(Field::new().selector("h1.entry-title"))
        .author(Field::new().selectors([".author vcard", ".fn"]))
        .content(Field::new().selectors([".grid-content", ".entry-content"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new())
}
