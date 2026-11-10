//! www.eonline.com

use crate::extractors::{CustomExtractor, Field};

pub fn extractor() -> CustomExtractor {
    CustomExtractor::new("www.eonline.com")
        .title(Field::new().selectors(["h1.article-detail__title", "h1.article__title"]))
        .author(Field::new().selectors([".article-detail__meta__author", ".entry-meta__author a"]))
        .date_published(
            Field::new()
                .attr(r#"meta[name="article:published_time"]"#, "value")
                .attr(r#"meta[itemprop="datePublished"]"#, "value"),
        )
        .lead_image_url(Field::new().attr(r#"meta[name="og:image"]"#, "value"))
        .content(
            Field::new()
                .group([".article-detail__main-content section"])
                .group([".post-content section, .post-content div.post-content__image"])
                .rename("div.post-content__image", "figure")
                .rename("div.post-content__image .image__credits", "figcaption"),
        )
}
