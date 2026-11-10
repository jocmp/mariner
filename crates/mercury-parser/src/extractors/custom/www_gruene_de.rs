//! www.gruene.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.gruene.de")
        .title(Field::new().selector("header h1"))
        .lead_image_url(Field::new().attr(r#"meta[property="og:image"]"#, "content"))
        .content(
            Field::new()
                .group(["section header", "section h2", "section p", "section ol"])
                .clean(["figcaption", "p[class]"]),
        )
}
