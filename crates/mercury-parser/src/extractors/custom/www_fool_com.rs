//! www.fool.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.fool.com")
        .title(Field::new().selector("h1"))
        .author(
            Field::new()
                .attr(r#"meta[name="author"]"#, "value")
                .selector(".author-inline .author-name"),
        )
        .date_published(Field::new().attr(r#"meta[name="date"]"#, "value"))
        .dek(
            Field::new()
                .attr(r#"meta[name="og:description"]"#, "value")
                .selector("header h2"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".tailwind-article-body", ".article-content"])
                .transform(".caption img", |node| {
                    // Upstream interpolates `undefined` when there is no src.
                    let src = node.attr("src");
                    let src = src.as_deref().unwrap_or("undefined");
                    node.parent()
                        .replace_with_html(format!(r#"<figure><img src="{src}"/></figure>"#));
                    None
                })
                .rename(".caption", "figcaption")
                .clean(["#pitch"]),
        )
}
