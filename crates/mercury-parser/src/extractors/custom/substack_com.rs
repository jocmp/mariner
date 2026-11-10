//! substack.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::outer_html;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("substack.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".available-content")
                .rename("div.captioned-image-container", "figure")
                .transform("div.image-link", |node| {
                    node.replace_with_html(outer_html(&node.select("img")));
                    None
                })
                .clean([
                    ".subscribe-widget",
                    ".subscription-widget-wrap",
                    ".subscription-widget-wrap-editor",
                    ".button-wrapper",
                    ".poll-embed",
                    ".share-dialog",
                ]),
        )
}
