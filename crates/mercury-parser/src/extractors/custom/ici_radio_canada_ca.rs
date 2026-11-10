//! ici.radio-canada.ca

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("ici.radio-canada.ca")
        .title(Field::new().attr(r#"meta[name="dc.title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="dc.creator"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="dc.date.created"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["article main", "article"])
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("ul", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    "header",
                    "nav",
                    "button",
                    "figcaption",
                    r#"[class*="adBox"]"#,
                    ".framed",
                ]),
        )
}
