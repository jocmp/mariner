//! www.dw.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.dw.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(".author-name .author-link"))
        .date_published(Field::new().attr(r#"meta[name="date"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(r#"[data-tracking-name="rich-text"]"#)
                .transform("img", |node| {
                    let template = node
                        .attr("data-url")
                        .filter(|s| !s.is_empty())
                        .or_else(|| node.attr("src"))
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    if template.contains("${formatId}") {
                        node.set_attr("src", &template.replacen("${formatId}", "6", 1));
                    }
                    None
                })
                .clean(["blockquote.tweet.embed"]),
        )
}
