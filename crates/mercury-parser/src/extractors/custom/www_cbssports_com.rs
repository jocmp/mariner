//! www.cbssports.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.cbssports.com")
        .title(Field::new().selectors([".Article-headline", ".article-headline"]))
        .author(Field::new().selectors([".ArticleAuthor-nameText", ".author-name"]))
        .date_published(
            Field::new()
                .attr(r#"meta[itemprop="datePublished"]"#, "value")
                .timezone("UTC"),
        )
        .dek(Field::new().selectors([".Article-subline", ".article-subline"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".article"))
}
