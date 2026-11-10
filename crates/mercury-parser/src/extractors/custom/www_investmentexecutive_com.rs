//! www.investmentexecutive.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.investmentexecutive.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector(r#"div[itemprop="author"]"#))
        .date_published(Field::new().attr(r#"meta[itemprop="datePublished"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("section.article-body")
                .clean([".hidden"]),
        )
}
