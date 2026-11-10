//! wikipedia.org

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("wikipedia.org")
        .content(
            Field::new()
                .selector("#mw-content-text")
                .default_cleaner(false)
                .transform(".infobox img", |node| {
                    let parent = node.ancestors(None).filter(".infobox");
                    // Only prepend the first image in .infobox
                    if parent.exists() && parent.children().filter("img").is_empty() {
                        parent.prepend_selection(node);
                    }
                    None
                })
                .rename(".infobox caption", "figcaption")
                .rename(".infobox", "figure")
                .clean([
                    ".mw-editsection",
                    "figure tr, figure td, figure tbody",
                    "#toc",
                    ".navbox",
                ]),
        )
        .author(Field::literal("Wikipedia Contributors"))
        .title(Field::new().selector("h2.title"))
        .date_published(Field::new().selector("#footer-info-lastmod"))
}
