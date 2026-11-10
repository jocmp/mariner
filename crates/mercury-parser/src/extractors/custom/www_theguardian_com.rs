//! www.theguardian.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.theguardian.com")
        .title(Field::new().selectors(["h1", ".content__headline"]))
        .author(Field::new().selectors([r#"address[data-link-name="byline"]"#, "p.byline"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selectors([r#"div[data-gu-name="standfirst"]"#, ".content__standfirst"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["#maincontent", ".content__article-body"])
                .clean([".hide-on-mobile", ".inline-icon"]),
        )
}
