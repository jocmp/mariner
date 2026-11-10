//! news.mynavi.jp

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("news.mynavi.jp")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selectors([
            "a.articleHeader_name",
            "main div.article-author a.article-author__name",
        ]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["div.article-body", "main article div"])
                .transform("img", |node| {
                    if let Some(src) = node.attr("data-original").filter(|s| !s.is_empty()) {
                        node.set_attr("src", &src);
                    }
                    None
                }),
        )
}
