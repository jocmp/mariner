//! www.tagesschau.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.tagesschau.de")
        .title(Field::new().selectors([".seitenkopf__headline--text", "title"]))
        .author(Field::new().selector(".authorline__author authorline__link:first-child"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="date"]"#, "value")
                .selector(".metatextline")
                .timezone("UTC"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector("article").clean([
            "[data-config]",
            ".seitenkopf__headline",
            ".authorline__author",
            ".metatextline",
        ]))
}
