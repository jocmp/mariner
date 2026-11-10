//! thoughtcatalog.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("thoughtcatalog.com")
        .title(
            Field::new()
                .selector("h1.title")
                .attr(r#"meta[name="og:title"]"#, "value"),
        )
        .author(Field::new().selectors([
            "cite a",
            "div.col-xs-12.article_header div.writer-container.writer-container-inline.writer-no-avatar h4.writer-name",
            "h1.writer-name",
        ]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".entry.post")
                .clean([".tc_mark", "figcaption"]),
        )
}
