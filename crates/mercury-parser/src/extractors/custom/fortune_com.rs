//! fortune.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("fortune.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().selector(".MblGHNMJ").timezone("UTC"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["picture", "article.row"])
                .selector("article.row"),
        )
}
