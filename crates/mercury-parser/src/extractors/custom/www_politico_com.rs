//! www.politico.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.politico.com")
        .title(Field::new().attr(r#"meta[name="og:title"]"#, "value"))
        .author(
            Field::new()
                .attr(r#"div[itemprop="author"] meta[itemprop="name"]"#, "value")
                .selectors([
                    ".story-meta__authors .vcard",
                    ".story-main-content .byline .vcard",
                ]),
        )
        .content(
            Field::new()
                .group([".story-text"])
                .selectors([".story-main-content", ".story-core"])
                .clean(["figcaption", ".story-meta", ".ad"]),
        )
        .date_published(
            Field::new()
                .attr(r#"time[itemprop="datePublished"]"#, "datetime")
                .attr(".story-meta__details time[datetime]", "datetime")
                .attr(".story-main-content .timestamp time[datetime]", "datetime")
                .timezone("America/New_York"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .dek(Field::new().attr(r#"meta[name="og:description"]"#, "value"))
}
