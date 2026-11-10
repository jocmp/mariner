//! thefederalistpapers.org

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("thefederalistpapers.org")
        .title(Field::new().selector("h1.entry-title"))
        .author(Field::new().selectors([".author-meta-title", "main span.entry-author-name"]))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(Field::new().selector(".content").clean([
            "header",
            ".article-sharing",
            ".after-article",
            ".type-commenting",
            ".more-posts",
            "p[style]",
        ]))
}
