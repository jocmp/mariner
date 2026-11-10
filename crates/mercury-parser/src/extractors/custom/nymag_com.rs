//! nymag.com

use dom_query::{Document, Selection};

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::inner_html_of;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("nymag.com")
        .content(
            Field::new()
                .selectors(["div.article-content", "section.body", "article.article"])
                .clean([".ad", ".single-related-story"])
                .rename("h1", "h2")
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
        .title(Field::new().selectors(["h1.lede-feature-title", "h1.headline-primary", "h1"]))
        .author(Field::new().selectors([".by-authors", ".lede-feature-author"]))
        .dek(Field::new().selector(".lede-feature-teaser"))
        .date_published(
            Field::new()
                .attr("time.article-timestamp[datetime]", "datetime")
                .selector("time.article-timestamp"),
        )
}
