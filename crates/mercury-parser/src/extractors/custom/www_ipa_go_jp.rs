//! www.ipa.go.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.ipa.go.jp")
        .title(Field::new().selector("h1"))
        .date_published(
            Field::new()
                .selector("p.ipar_text_right")
                .format("YYYY年M月D日")
                .timezone("Asia/Tokyo"),
        )
        .content(
            Field::new()
                .selector("#ipar_main")
                .default_cleaner(false)
                .clean(["p.ipar_text_right"]),
        )
}
