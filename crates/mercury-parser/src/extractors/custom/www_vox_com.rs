//! www.vox.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.vox.com")
        .title(Field::new().selector("h1.c-page-title"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().selector(".p-dek"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group(["figure.e-image--hero", ".c-entry-content"])
                .selector(".c-entry-content")
                .transform("figure .e-image__image noscript", |node| {
                    let img_html = inner_html_of(node);
                    node.ancestors(None)
                        .filter(".e-image__image")
                        .select(".c-dynamic-image")
                        .replace_with_html(img_html);
                    None
                })
                .rename("figure .e-image__meta", "figcaption"),
        )
}
