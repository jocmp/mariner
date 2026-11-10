//! www.publickey1.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.publickey1.jp")
        .title(Field::new().selector("h1"))
        .author(Field::new().selectors([".bloggerinchief p:first-of-type", "#subcol p:has(img)"]))
        .date_published(
            Field::new()
                .selector("div.pubdate")
                .format("YYYY年MM月DD日")
                .timezone("Asia/Tokyo"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#maincol")
                .default_cleaner(false)
                .clean(["#breadcrumbs", "div.sbm", "div.ad_footer"]),
        )
}
