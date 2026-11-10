//! abcnews.go.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("abcnews.go.com")
        .title(Field::new().selectors([
            r#"div[class*="Article_main__body"] h1"#,
            ".article-header h1",
        ]))
        .author(
            Field::new()
                .selectors([".ShareByline span:nth-child(2)", ".authors"])
                .clean([".author-overlay", ".by-text"]),
        )
        .date_published(
            Field::new()
                .selectors([".ShareByline", ".timestamp"])
                .format("MMMM D, YYYY h:mm a")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors(["article", ".article-copy"]))
}
