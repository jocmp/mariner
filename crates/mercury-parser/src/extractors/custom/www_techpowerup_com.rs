//! www.techpowerup.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.techpowerup.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".byline address"))
        .date_published(Field::new().attr(".byline time[datetime]", "datetime"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".contnt")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean(["header", "footer"]),
        )
        .next_page_url(Field::new().attr(".nextpage-bottom", "href"))
}
