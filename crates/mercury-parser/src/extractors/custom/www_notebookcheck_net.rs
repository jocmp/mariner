//! www.notebookcheck.net

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.notebookcheck.net")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector(".intro-author a"))
        .date_published(
            Field::new()
                .attr(".intro-author time", "datetime")
                .timezone("GMT"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("#content")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h4", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    ".ttcl_3",
                    ".socialarea",
                    ".tx-nbc2fe-relatedarticles",
                    "aside",
                ]),
        )
}
