//! www.al.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.al.com")
        .title(Field::new().attr(r#"meta[name="title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="article_author"]"#, "value"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article_date_original"]"#, "value")
                .timezone("EST"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".entry-content"))
}
