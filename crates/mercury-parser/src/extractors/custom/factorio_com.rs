//! factorio.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("factorio.com")
        .title(Field::new().selector("title"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".blog-post", "div:nth-child(2)"])
                .transform("h3", |node| {
                    let author = node.select("author");
                    let text = author.text();
                    if !text.is_empty() {
                        node.after_html(format!("<p>{text}</p>"));
                        author.remove();
                    }
                    None
                })
                .clean([".logo-expansion-space-age"]),
        )
}
