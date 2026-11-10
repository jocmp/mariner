//! wccftech.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("wccftech.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector("div.meta a:first-of-type"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="pub_date"]"#, "value")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".content").clean([".democracy"]))
}
