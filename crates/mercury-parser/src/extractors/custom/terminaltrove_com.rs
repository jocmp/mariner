//! terminaltrove.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("terminaltrove.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("main").clean([
            ".share-badge",
            ".modal",
            ".modal-toggle",
            ".sr-only",
            ".premium-sponsor-featured",
        ]))
}
