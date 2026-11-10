//! www.polygon.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.polygon.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("article")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("h3", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .transform("img", |node| {
                    let srcset = node.attr("srcset");
                    let srcset = srcset.as_deref().unwrap_or("");
                    let src = srcset.split(',').next().unwrap_or("");
                    if !src.is_empty() {
                        node.parent().replace_with_html(format!(
                            r#"<figure><img srcset="{srcset}" src="{src}"/></figure>"#
                        ));
                    }
                    None
                })
                .clean([
                    "cite",
                    ".duet--ad--native-ad-rail",
                    ".duet--layout--rail",
                    ".duet--article--table-of-contents",
                ]),
        )
}
