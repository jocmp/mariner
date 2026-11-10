//! www.moongift.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.moongift.jp")
        .title(Field::new().selector("h1.title a"))
        .date_published(
            Field::new()
                .selector("ul.meta li:not(.social):first-of-type")
                .timezone("Asia/Tokyo"),
        )
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("#main").clean(["ul.mg_service.cf"]))
}
