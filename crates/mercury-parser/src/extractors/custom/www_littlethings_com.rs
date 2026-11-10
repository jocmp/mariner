//! www.littlethings.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.littlethings.com")
        .title(Field::new().selectors([r#"h1[class*="PostHeader"]"#, "h1.post-title"]))
        .author(
            Field::new()
                .selector(r#"div[class^="PostHeader__ScAuthorNameSection"]"#)
                .attr(r#"meta[name="author"]"#, "value"),
        )
        .content(Field::new().selectors([
            r#"section[class*="PostMainArticle"]"#,
            ".mainContentIntro",
            ".content-wrapper",
        ]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
}
