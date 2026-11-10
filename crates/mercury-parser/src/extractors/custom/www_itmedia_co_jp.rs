//! www.itmedia.co.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.itmedia.co.jp")
        .supported_domains([
            "www.atmarkit.co.jp",
            "techtarget.itmedia.co.jp",
            "nlab.itmedia.co.jp",
        ])
        .title(Field::new().selector("#cmsTitle h1"))
        .author(Field::new().selector("#byline"))
        .date_published(Field::new().attr(r#"meta[name="article:modified_time"]"#, "value"))
        .dek(Field::new().selector("#cmsAbstract h2"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#cmsBody")
                .default_cleaner(false)
                .clean(["#snsSharebox"]),
        )
}
