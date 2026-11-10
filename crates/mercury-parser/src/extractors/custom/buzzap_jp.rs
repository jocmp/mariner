//! buzzap.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("buzzap.jp")
        .title(Field::new().selector("h1.entry-title"))
        .date_published(Field::new().attr("time.entry-date", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.ctiframe").default_cleaner(false))
}
