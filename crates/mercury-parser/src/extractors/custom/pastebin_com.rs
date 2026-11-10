//! pastebin.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("pastebin.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selectors([".username", ".paste_box_line2 .t_us + a"]))
        .date_published(
            Field::new()
                .selectors([".date", ".paste_box_line2 .t_da + span"])
                .timezone("America/New_York")
                .format("MMMM D, YYYY"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".source", "#selectable .text"])
                .rename("ol", "div")
                .rename("li", "p"),
        )
}
