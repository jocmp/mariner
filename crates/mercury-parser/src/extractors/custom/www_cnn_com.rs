//! www.cnn.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.cnn.com")
        .title(Field::new().selectors(["h1.pg-headline", "h1"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".media__video--thumbnail", ".zn-body-text"])
                .selectors([".zn-body-text", r#"div[itemprop="articleBody"]"#])
                .transform(
                    ".zn-body__paragraph, .el__leafmedia--sourced-paragraph",
                    |node| {
                        if !inner_html_of(node).is_empty() {
                            return Some("p".to_string());
                        }

                        None
                    },
                )
                .transform(".zn-body__paragraph", |node| {
                    // Upstream guards this with `$node.has('a')`, which is always truthy.
                    if node.text().trim() == node.select("a").text().trim() {
                        node.remove();
                    }
                    None
                })
                .rename(".media__video--thumbnail", "figure"),
        )
}
