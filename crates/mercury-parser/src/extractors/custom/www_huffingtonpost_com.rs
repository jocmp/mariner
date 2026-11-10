//! www.huffingtonpost.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.huffingtonpost.com")
        .title(Field::new().selector("h1.headline__title"))
        .author(Field::new().selector("span.author-card__details__name"))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:modified_time"]"#, "value")
                .attr(r#"meta[name="article:published_time"]"#, "value"),
        )
        .dek(Field::new().selector("h2.headline__subtitle"))
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selector("div.entry__body")
                .default_cleaner(false)
                .clean([
                    ".pull-quote",
                    ".tag-cloud",
                    ".embed-asset",
                    ".below-entry",
                    ".entry-corrections",
                    "#suggested-story",
                ]),
        )
}
