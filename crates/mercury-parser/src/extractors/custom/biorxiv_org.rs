//! biorxiv.org

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("biorxiv.org")
        .title(Field::new().selector("h1#page-title"))
        .author(
            Field::new()
                .selector("div.highwire-citation-biorxiv-article-top > div.highwire-cite-authors"),
        )
        .content(Field::new().selector("div#abstract-1"))
}
