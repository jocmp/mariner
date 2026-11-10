//! www.cnbc.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.cnbc.com")
        .title(Field::new().selectors(["h1.title", "h1.ArticleHeader-headline"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([
            "div#article_body.content",
            "div.story",
            "div.ArticleBody-articleBody",
        ]))
}
