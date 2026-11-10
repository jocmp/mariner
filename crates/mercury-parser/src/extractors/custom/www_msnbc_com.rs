//! www.msnbc.com

use dom_query::Selection;

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.msnbc.com")
        .title(Field::new().selectors(["h1", "h1.is-title-pane"]))
        .author(Field::new().selectors([".byline-name", ".author"]))
        .date_published(
            Field::new()
                .attr(r#"meta[itemprop="datePublished"]"#, "value")
                .attr(r#"meta[name="DC.date.issued"]"#, "value"),
        )
        .dek(Field::new().attr(r#"meta[name="description"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".article-body__content", ".pane-node-body"])
                .transform(".pane-node-body", |node| {
                    // Upstream reads the first `lead_image_url` selector from the whole document.
                    let root = Selection::from(node.nodes().first()?.tree.root());
                    if let Some(src) = root
                        .select(r#"meta[name="og:image"]"#)
                        .attr("value")
                        .filter(|s| !s.is_empty())
                    {
                        node.prepend_html(format!(r#"<img src="{src}" />"#));
                    }
                    None
                }),
        )
}
