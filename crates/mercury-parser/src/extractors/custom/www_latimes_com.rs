//! www.latimes.com

use crate::extractors::{CustomExtractor, Field};
use crate::utils::dom::outer_html;

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.latimes.com")
        .title(Field::new().selectors(["h1.headline", ".trb_ar_hl"]))
        .author(
            Field::new()
                .selector(r#"a[data-click="standardBylineAuthorName"]"#)
                .attr(r#"meta[name="author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr(r#"meta[itemprop="datePublished"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".page-article-body", ".trb_ar_main"])
                .transform(".trb_ar_la", |node| {
                    node.replace_with_html(outer_html(&node.select("figure")));
                    None
                })
                .clean([".trb_ar_by", ".trb_ar_cr"]),
        )
}
