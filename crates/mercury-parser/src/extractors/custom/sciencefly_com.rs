//! sciencefly.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("sciencefly.com")
        .title(Field::new().selectors([".entry-title", ".cb-entry-title", ".cb-single-title"]))
        .author(Field::new().selectors(["div.cb-author", "div.cb-author-title"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new())
        .lead_image_url(Field::new().attr("div.theiaPostSlider_slides img", "src"))
        .content(Field::new().selector("div.theiaPostSlider_slides"))
}
