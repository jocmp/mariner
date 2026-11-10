//! phpspot.org

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("phpspot.org")
        .title(Field::new().selector("h3.hl"))
        .date_published(
            Field::new()
                .selector("h4.hl")
                .format("YYYY年M月D日")
                .timezone("Asia/Tokyo"),
        )
        .content(
            Field::new()
                .selector("div.entrybody")
                .default_cleaner(false),
        )
}
