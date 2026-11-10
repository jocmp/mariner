//! www.elecom.co.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.elecom.co.jp")
        .title(Field::new().selector("title"))
        .date_published(
            Field::new()
                .selector("p.section-last")
                .format("YYYY.M.D")
                .timezone("Asia/Tokyo"),
        )
        .content(
            Field::new()
                .selector("td.TableMain2")
                .default_cleaner(false)
                .transform("table", |node| {
                    node.set_attr("width", "auto");
                    None
                }),
        )
}
