//! gothamist.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("gothamist.com")
        .supported_domains([
            "chicagoist.com",
            "laist.com",
            "sfist.com",
            "shanghaiist.com",
            "dcist.com",
        ])
        .title(Field::new().selectors(["h1", ".entry-header h1"]))
        .author(
            Field::new().selectors([".article-metadata:nth-child(3) .byline-author", ".author"]),
        )
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .selectors(["abbr", "abbr.published"])
                .timezone("America/New_York"),
        )
        .dek(Field::new())
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .selectors([".article-body", ".entry-body"])
                .rename("div.image-none", "figure")
                .rename(".image-none i", "figcaption")
                .rename("div.image-left", "figure")
                .rename(".image-left i", "figcaption")
                .rename("div.image-right", "figure")
                .rename(".image-right i", "figcaption")
                .clean([
                    ".image-none br",
                    ".image-left br",
                    ".image-right br",
                    ".galleryEase",
                ]),
        )
}
