//! forward.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("forward.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .selectors([".post-author a", ".author-name"])
                .attr(r#"meta[name="sailthru.author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr(r#"meta[name="date"]"#, "value"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".content-container article")
                .group([".post-item-media-wrap", ".post-item p"])
                .clean([".post-author", ".donate-box", ".message", ".subtitle"]),
        )
}
