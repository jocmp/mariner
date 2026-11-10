//! 247sports.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("247sports.com")
        .title(Field::new().selectors(["title", "article header h1"]))
        .author(Field::new().selectors([".article-cnt__author", ".author"]))
        .date_published(Field::new().attr("time[data-published]", "data-published"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selectors([".article-body", "section.body.article"]))
}
