//! www.thepoliticalinsider.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.thepoliticalinsider.com")
        .title(Field::new().attr(r#"meta[name="sailthru.title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="sailthru.author"]"#, "value"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="sailthru.date"]"#, "value")
                .timezone("America/New_York"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div#article-body"))
}
