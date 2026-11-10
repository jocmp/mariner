//! mashable.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("mashable.com")
        .title(Field::new().selectors(["header h1", "h1.title"]))
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector("span.author_name a"),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["#article", "section.article-content.blueprint"])
                .rename(".image-credit", "figcaption"),
        )
}
