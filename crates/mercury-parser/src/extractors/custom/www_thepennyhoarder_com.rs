//! www.thepennyhoarder.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.thepennyhoarder.com")
        .title(Field::new().attr(r#"meta[name="dcterms.title"]"#, "value"))
        .author(Field::new().attr(r#"link[rel="author"]"#, "title"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".post-img", ".post-text"])
                .selectors([".post-text", ".single-post-content-inner"]),
        )
}
