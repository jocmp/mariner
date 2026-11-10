//! arstechnica.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("arstechnica.com")
        .title(Field::new().selectors(["title", "h1"]))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".post-content", "main"])
                .transform("img", |node| {
                    node.remove_attr("width");
                    node.remove_attr("sizes");
                    None
                })
                .clean([
                    "header",
                    ".upper-deck__text",
                    ".text-settings-dropdown-story",
                ]),
        )
}
