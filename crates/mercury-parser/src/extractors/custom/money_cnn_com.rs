//! money.cnn.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("money.cnn.com")
        .title(Field::new().selector(".article-title"))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selector(".byline a"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="date"]"#, "value")
                .timezone("GMT"),
        )
        .dek(Field::new().selector("#storytext h2"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#storytext")
                .clean([".inStoryHeading"]),
        )
}
