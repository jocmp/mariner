//! www.phoronix.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.phoronix.com")
        .title(Field::new().selectors(["article h1", "article header"]))
        .author(Field::new().selector(".author a:first-child"))
        .date_published(
            Field::new()
                .selector(".author")
                .format("D MMMM YYYY at hh:mm")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".content", "article"])
                .default_cleaner(false)
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                }),
        )
}
