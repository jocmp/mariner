//! www.rbbtoday.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.rbbtoday.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector(".writer.writer-name"))
        .date_published(Field::new().attr("header time", "datetime"))
        .dek(
            Field::new()
                .attr(r#"meta[name="description"]"#, "value")
                .selector(".arti-summary"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".arti-content").clean([".arti-giga"]))
}
