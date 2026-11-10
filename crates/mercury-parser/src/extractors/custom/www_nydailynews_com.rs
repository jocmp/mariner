//! www.nydailynews.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.nydailynews.com")
        .title(Field::new().selectors(["h1.headline", "h1#ra-headline"]))
        .author(
            Field::new()
                .selector(".article_byline span")
                .attr(r#"meta[name="parsely-author"]"#, "value"),
        )
        .date_published(
            Field::new()
                .selector("time")
                .attr(r#"meta[name="sailthru.date"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors(["article", "article#ra-body"])
                .clean([
                    "dl#ra-tags",
                    ".ra-related",
                    "a.ra-editor",
                    "dl#ra-share-bottom",
                ]),
        )
}
