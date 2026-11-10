//! www.miamiherald.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.miamiherald.com")
        .title(Field::new().selector("h1.title"))
        .date_published(
            Field::new()
                .selector("p.published-date")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.dateline-storybody"))
}
