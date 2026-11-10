//! news.pts.org.tw

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("news.pts.org.tw")
        .title(Field::new().selector("h1.article-title"))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "content")
                .attr(r#"meta[name="author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[property="article:published_time"]"#, "content")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .dek(
            Field::new()
                .attr(r#"meta[name="description"]"#, "content")
                .attr(r#"meta[name="description"]"#, "value"),
        )
        .lead_image_url(
            Field::new()
                .attr(r#"meta[property="og:image"]"#, "content")
                .attr(r#"meta[name="og:image"]"#, "value"),
        )
        .content(
            Field::new()
                .selectors([".post-article", ".article-content"])
                .clean([".articleimg", "ul"]),
        )
}
