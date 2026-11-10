//! deadline.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("deadline.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector("section.author h2"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.a-article-grid__main.pmc-a-grid article.pmc-a-grid-item")
                .transform(".embed-twitter", |node| {
                    node.replace_with_html(inner_html_of(node));
                    None
                })
                .clean(["figcaption"]),
        )
}
