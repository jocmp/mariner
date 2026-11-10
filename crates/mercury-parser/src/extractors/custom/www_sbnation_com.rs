//! www.sbnation.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.sbnation.com")
        .title(Field::new().selector("h1.c-page-title"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selectors(["p.c-entry-summary.p-dek", "h2.c-entry-summary.p-dek"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.c-entry-content"))
}
