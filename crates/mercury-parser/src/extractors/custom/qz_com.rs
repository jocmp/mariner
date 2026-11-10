//! qz.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("qz.com")
        .title(Field::new().selector("article header h1"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("time[datetime]", "datetime"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(r#"meta[property="og:image"]"#, "content")
                .attr(r#"meta[name="twitter:image"]"#, "content"),
        )
        .content(Field::new().selector("#article-content"))
}
