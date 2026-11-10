//! epaper.zeit.de

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("epaper.zeit.de")
        .title(Field::new().selector("p.title"))
        .author(Field::new().selector(".article__author"))
        .excerpt(Field::new().selector("subtitle"))
        .content(
            Field::new()
                .selector(".article")
                .rename("p.title", "h1")
                .rename(".article__author", "p")
                .rename("byline", "p")
                .rename("linkbox", "p")
                .clean(["image-credits", "box[type=citation]"]),
        )
}
