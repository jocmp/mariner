//! www.ndtv.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.ndtv.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("h1.entry-title"),
        )
        .author(Field::new().selector(r#"span[itemprop="author"] span[itemprop="name"]"#))
        .date_published(Field::new().attr(r#"span[itemprop="dateModified"]"#, "content"))
        .dek(Field::new().selector("h2"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector(r#"div[itemprop="articleBody"]"#)
                .transform(".place_cont", |node| {
                    if node.ancestors(None).filter("p").is_empty() {
                        // Upstream removes the node and prepends it to the next sibling <p>; the
                        // node is dropped when there is no such sibling.
                        let next_sibling = node.next_sibling().filter("p");
                        next_sibling.prepend_selection(node);
                    }
                    None
                })
                .clean([
                    ".highlghts_Wdgt",
                    ".ins_instory_dv_caption",
                    "input",
                    "._world-wrapper .mt20",
                ]),
        )
}
