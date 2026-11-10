//! www.bloomberg.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.bloomberg.com")
        .title(Field::new().selectors([
            ".lede-headline",
            "h1.article-title",
            r#"h1[class^="headline"]"#,
            "h1.lede-text-only__hed",
        ]))
        .author(
            Field::new()
                .attr(r#"meta[name="parsely-author"]"#, "value")
                .selectors([
                    ".byline-details__link",
                    ".bydek",
                    ".author",
                    r#"p[class*="author"]"#,
                ]),
        )
        .date_published(
            Field::new()
                .attr("time.published-at", "datetime")
                .attr("time[datetime]", "datetime")
                .attr(r#"meta[name="date"]"#, "value")
                .attr(r#"meta[name="parsely-pub-date"]"#, "value")
                .attr(r#"meta[name="parsely-pub-date"]"#, "content"),
        )
        .dek(Field::new())
        .lead_image_url(
            Field::new()
                .attr(r#"meta[name="og:image"]"#, "value")
                .attr(r#"meta[name="og:image"]"#, "content"),
        )
        .content(
            Field::new()
                .selectors([".article-body__content", ".body-content"])
                .group(["section.copy-block"])
                .selector(".body-copy")
                .clean([".inline-newsletter", ".page-ad"]),
        )
}
