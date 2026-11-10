//! www.ilfattoquotidiano.it

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.ilfattoquotidiano.it")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .selector(".ifq-post__author .ifq-news-meta__author-name")
                .clean(["span"]),
        )
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([".ifq-post__content", "article"]))
}
