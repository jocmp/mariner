//! sg.news.yahoo.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("sg.news.yahoo.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("title"),
        )
        .author(Field::new().selectors([".caas-attr-provider", r#"meta[name="author"]"#]))
        .date_published(
            Field::new()
                .selectors([
                    "time[datetime]",
                    r#"meta[property="article:published_time"]"#,
                ])
                .timezone("UTC"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".caas-body-content", "article"])
                .clean([
                    ".caas-header",
                    ".caas-logo",
                    ".caas-title-wrapper",
                    "button",
                    ".advertisement",
                    // Upstream's `.sda-*`, which cheerio reads as `.sda-` plus a universal
                    // selector; dom_query rejects that spelling.
                    ".sda-",
                    r#"[data-content="Advertisement"]"#,
                ]),
        )
}
