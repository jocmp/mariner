//! japan.zdnet.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("japan.zdnet.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().attr(r#"meta[name="cXenseParse:author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("div.article_body"))
}
