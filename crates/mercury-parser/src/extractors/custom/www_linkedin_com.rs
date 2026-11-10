//! www.linkedin.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.linkedin.com")
        .title(Field::new().selectors([".article-title", "h1"]))
        .author(
            Field::new()
                .selector(".main-author-card h3")
                .attr(r#"meta[name="article:author"]"#, "value")
                .selector(".entity-name a[rel=author]"),
        )
        .date_published(
            Field::new()
                .selector(".base-main-card__metadata")
                .attr(r#"time[itemprop="datePublished"]"#, "datetime")
                .timezone("America/Los_Angeles"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".article-content__body")
                .group(["header figure", ".prose"])
                .selector(".prose")
                .clean([".entity-image"]),
        )
}
