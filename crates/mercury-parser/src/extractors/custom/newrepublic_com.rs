//! newrepublic.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("newrepublic.com")
        .title(Field::new().selector("h1.article-headline"))
        .author(Field::new().selector("span.AuthorList"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector("h2.article-subhead"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().group(["div.article-body"]).clean(["aside"]))
}
