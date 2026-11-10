//! www.qbitai.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.qbitai.com")
        .title(Field::new().selectors(["title", "h1"]))
        .content(
            Field::new()
                .selector(".article")
                .transform(".zhaiyao", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([".article_info"]),
        )
}
