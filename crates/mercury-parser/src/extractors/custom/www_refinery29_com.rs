//! www.refinery29.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.refinery29.com")
        .title(Field::new().selector("h1.title"))
        .author(Field::new().selector(".contributor"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="sailthru.date"]"#, "value")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".full-width-opener", ".article-content"])
                .selectors([".article-content", ".body"])
                .transform("div.loading noscript", |node| {
                    let img_html = inner_html_of(node);
                    node.ancestors(None)
                        .filter(".loading")
                        .replace_with_html(img_html);
                    None
                })
                .rename(".section-image", "figure")
                .rename(".section-image .content-caption", "figcaption")
                .rename(".section-text", "p")
                .clean([".story-share"]),
        )
}
