//! www.sanwa.co.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.sanwa.co.jp")
        .title(Field::new().selector("#newsContent h1"))
        .date_published(
            Field::new()
                .selector("dl.date")
                .format("YYYY.M.D")
                .timezone("Asia/Tokyo"),
        )
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#newsContent")
                .default_cleaner(false)
                .clean(["#smartphone", "div.sns_box", "div.contentFoot"]),
        )
}
