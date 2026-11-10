//! getnews.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("getnews.jp")
        .title(Field::new().selector("article h1"))
        .author(
            Field::new()
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector("span.prof"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr("ul.cattag-top time", "datetime"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.post-bodycopy"))
}
