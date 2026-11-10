//! www.vortez.net

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.vortez.net")
        .title(Field::new().selector("title"))
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .next_page_url(Field::new().selector(".pagelink:nth-child(2) > a"))
        .content(
            Field::new()
                .selectors([".main-content", ".the-article-content"])
                .rename("strong", "p")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([".article-header", ".panel-title", "select", "br"]),
        )
}
