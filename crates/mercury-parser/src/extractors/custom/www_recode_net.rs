//! www.recode.net

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.recode.net")
        .title(Field::new().selector("h1.c-page-title"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector("h2.c-entry-summary.p-dek"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["figure.e-image--hero", ".c-entry-content"])
                .selector(".c-entry-content"),
        )
}
