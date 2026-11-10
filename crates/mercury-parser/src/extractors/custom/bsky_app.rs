//! bsky.app

use dom_query::{Document, Selection};

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("bsky.app")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .lead_image_url(
            Field::new()
                .attr(r#"meta[property="og:image"]"#, "content")
                .attr(r#"meta[name="og:image"]"#, "value"),
        )
        .content(
            Field::new()
                .selector("noscript")
                .transform("noscript", |node| {
                    // Upstream re-parses the noscript markup and searches below its
                    // top-level elements.
                    let parsed = Document::fragment(inner_html_of(node));
                    let summary = Selection::from(parsed.html_root())
                        .children()
                        .select("#bsky_post_text");
                    node.replace_with_html(inner_html_of(&summary));
                    None
                }),
        )
}
