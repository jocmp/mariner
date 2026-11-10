//! www.motorsport.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.motorsport.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().selector(r#".msnt-author-toolbar a[href*="/info/about-us/"]"#))
        .date_published(Field::new().attr(r#"meta[name="datePublished"]"#, "value"))
        .dek(Field::new().selector("h2.text-article-description"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".ms-article-content")
                .transform("h2", |node| {
                    node.set_attr("class", "mercury-parser-keep");
                    None
                })
                .clean([
                    "msnt-survey-promo",
                    ".article-fullwidth-gallery_item ~ .article-fullwidth-gallery_item",
                    ".ms-inarticle-widgets",
                    ".relatedContent",
                    ".ms-apb",
                    ".ms-ap-native",
                    ".outstream_partner",
                ]),
        )
}
