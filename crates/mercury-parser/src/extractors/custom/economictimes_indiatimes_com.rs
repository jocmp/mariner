//! economictimes.indiatimes.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("economictimes.indiatimes.com")
        .title(
            Field::new()
                .selector("title")
                .attr(r#"meta[name="og:title"]"#, "value"),
        )
        .author(Field::new().selector(r#"a[rel="author"]"#))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("article").clean(["span.imgAgency"]))
}
