//! jvndb.jvn.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("jvndb.jvn.jp")
        .title(Field::new().selector("title"))
        .date_published(
            Field::new()
                .selector("div.modifytxt:nth-child(2)")
                .format("YYYY/M/D")
                .timezone("Asia/Tokyo"),
        )
        .content(Field::new().selector("#news-list").default_cleaner(false))
}
