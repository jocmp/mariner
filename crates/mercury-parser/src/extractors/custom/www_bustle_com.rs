//! www.bustle.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.bustle.com")
        .title(Field::new().selectors(["h1", "h1.post-page__title"]))
        .author(Field::new().selectors([r#"a[href*="profile"]"#, "div.content-meta__author"]))
        .date_published(Field::new().attr("time", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors(["article", ".post-page__body"]))
}
