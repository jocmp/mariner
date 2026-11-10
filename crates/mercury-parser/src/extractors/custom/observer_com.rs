//! observer.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("observer.com")
        .title(Field::new().selector("h1.entry-title"))
        .author(Field::new().selectors([".author", ".vcard"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector("h2.dek"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.entry-content"))
}
