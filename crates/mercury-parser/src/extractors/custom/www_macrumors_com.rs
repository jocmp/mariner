//! www.macrumors.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.macrumors.com")
        .title(Field::new().selectors(["h1", "h1.title"]))
        .author(Field::new().selectors([r#"article a[rel="author"]"#, ".author-url"]))
        .date_published(
            Field::new()
                .attr("time", "datetime")
                .timezone("America/Los_Angeles"),
        )
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors(["article", ".article"]))
}
