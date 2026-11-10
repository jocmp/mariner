//! www.nbcnews.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.nbcnews.com")
        .title(Field::new().selectors(["div.article-hero-headline h1", "div.article-hed h1"]))
        .author(Field::new().selectors([
            "div.article-inline-byline span.byline-name",
            "span.byline_author",
        ]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published"]"#, "value")
                .attr(
                    ".flag_article-wrapper time.timestamp_article[datetime]",
                    "datetime",
                )
                .selector(".flag_article-wrapper time"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors(["div.article-body__content", "div.article-body"]))
}
