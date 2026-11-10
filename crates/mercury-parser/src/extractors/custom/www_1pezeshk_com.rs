//! www.1pezeshk.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.1pezeshk.com")
        .title(
            Field::new()
                .attr(r#"meta[name="og:title"]"#, "value")
                .selector("h1.post-title"),
        )
        .author(Field::new().attr(r#"meta[name="author"]"#, "value"))
        .date_published(Field::new().attr(r#"meta[name="article:published_time"]"#, "value"))
        .lead_image_url(Field::new().attr(".featured-area img", "src"))
        .content(
            Field::new()
                .selector("article > .entry-content")
                .transform("img", |_| {
                    // Upstream assigns `$node.src`, a property on the cheerio wrapper rather than
                    // the attribute, so the DOM is never changed.
                    None
                }),
        )
}
