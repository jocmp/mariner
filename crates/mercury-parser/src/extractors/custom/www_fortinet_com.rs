//! www.fortinet.com

use dom_query::{Document, Selection};

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.fortinet.com")
        .title(Field::new().selector("h1"))
        .author(Field::new().selector(".b15-blog-meta__author"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.responsivegrid.aem-GridColumn.aem-GridColumn--default--12")
                .transform("noscript", |node| {
                    let noscript_html = inner_html_of(node);
                    if noscript_html.is_empty() {
                        return None;
                    }

                    let parsed = Document::fragment(noscript_html);
                    let children = Selection::from(parsed.html_root()).select("*");
                    let is_img = children
                        .nodes()
                        .first()
                        .and_then(|n| n.node_name())
                        .is_some_and(|name| name.eq_ignore_ascii_case("img"));
                    if children.length() == 1 && is_img {
                        return Some("figure".to_string());
                    }

                    None
                }),
        )
}
