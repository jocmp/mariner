//! www.infoq.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.infoq.com")
        .title(Field::new().selector("h1.heading"))
        .author(Field::new().selector("div.widget.article__authors"))
        .date_published(
            Field::new()
                .selector(".article__readTime.date")
                .format("YYYY年M月D日")
                .timezone("Asia/Tokyo"),
        )
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.article__data")
                .default_cleaner(false),
        )
}
