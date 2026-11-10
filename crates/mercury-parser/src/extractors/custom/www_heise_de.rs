//! www.heise.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.heise.de")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="date"]"#, "value"))
        .dek(Field::new().selector(".a-article-header__lead"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".article-layout__content")
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    ".ad-mobile-group-1",
                    ".branding",
                    r#"[data-component="RecommendationBox"]"#,
                ]),
        )
}
