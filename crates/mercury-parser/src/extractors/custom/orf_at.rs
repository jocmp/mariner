//! orf.at

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("orf.at")
        .title(Field::new().selector("title"))
        .date_published(Field::new().attr(r#"meta[name="dc.date"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("main").clean([".story-meta"]))
}
