//! people.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("people.com")
        .title(
            Field::new()
                .selector(".article-header h1")
                .attr(r#"meta[name="og:title"]"#, "value"),
        )
        .author(
            Field::new()
                .attr(r#"meta[name="sailthru.author"]"#, "value")
                .selector("a.author.url.fn"),
        )
        .date_published(
            Field::new()
                .selector(".mntl-attribution__item-date")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new().selector(".article-header h2"))
        .content(Field::new().selectors([
            r#"div[class^="loc article-content"]"#,
            "div.article-body__inner",
        ]))
}
