//! www.si.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.si.com")
        .title(Field::new().selectors(["h1", "h1.headline"]))
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="published"]"#, "value")
                .timezone("America/New_York"),
        )
        .dek(Field::new().selector(".m-detail-header--dek"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(".m-detail--body")
                .group(["p", ".marquee_large_2x", ".component.image"])
                .transform("noscript", |node| {
                    let children = node.children();
                    let is_img = children
                        .nodes()
                        .first()
                        .and_then(|n| n.node_name())
                        .is_some_and(|name| &*name == "img");
                    if children.length() == 1 && is_img {
                        return Some("figure".to_string());
                    }

                    None
                })
                .clean([".inline-thumb,.primary-message,.description,.instructions"]),
        )
}
