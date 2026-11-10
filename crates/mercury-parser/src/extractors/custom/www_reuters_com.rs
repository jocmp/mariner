//! www.reuters.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.reuters.com")
        .title(Field::new().selectors([
            r#"h1[class*="ArticleHeader-headline-"]"#,
            "h1.article-headline",
        ]))
        .author(
            Field::new()
                .attr(r#"meta[name="og:article:author"]"#, "value")
                .selector(".author"),
        )
        .date_published(Field::new().attr(r#"meta[name="og:article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["div.ArticleBodyWrapper", "#article-text"])
                .rename(".article-subtitle", "h4")
                .clean([
                    r#"div[class^="ArticleBody-byline-container-"]"#,
                    "#article-byline .author",
                ]),
        )
}
