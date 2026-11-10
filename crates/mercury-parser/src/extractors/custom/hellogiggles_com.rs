//! hellogiggles.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("hellogiggles.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector(".title"),
        )
        .author(Field::new().selectors([".byline-wrapper span.author_name", ".author-link"]))
        .date_published(
            Field::new()
                .attr(r#"meta[property="article:published_time"]"#, "content")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([".main-content", ".entry-content"]))
}
